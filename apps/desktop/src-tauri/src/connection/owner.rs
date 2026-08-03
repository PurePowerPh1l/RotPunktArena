//! Owner thread: Startup Nuclear (Known once) + Badge/Setup Nuclear.
//!
//! Start: Known BD_ADDR → genau 1× Full Nuclear (Forget → Pair → RFCOMM).
//!        kein Known → NeedsTarget. Fail → Idle (Badge kann erneut Nuclear).
//! Link lost → Idle (kein Auto-Nuclear).
//!
//! File length: command dispatch + Nuclear stay colocated on one thread
//! so generation bumps and socket ownership stay single-flight.

use super::command::ConnectionCommand;
use super::connect_policy::{BondLookup, ConnectOrigin, ConnectPhase};
use super::diag::{self, DiagEvent};
use super::event::{ConnectionEvent, ConnectionEventKind};
use super::nuclear::{self, ForgetScope};
use super::persist::{clear_known_target, save_known_target};
use super::shared::SharedState;
use super::sink::SinkFanout;
use super::status::ConnectionStatus;
use super::timing::{CMD_IDLE, POST_CONNECT_SETTLE, READ_SLICE};
use crate::protocol::RedDotStreamParser;
use crate::transport::rfcomm::auth_hook;
use crate::transport::rfcomm::discovery::bond_state;
use crate::transport::rfcomm::spp_com;
use crate::transport::rfcomm::target::RfcommTarget;
use crate::transport::rfcomm::ByteTransport;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) struct Owner {
    pub(crate) data_dir: PathBuf,
    pub(crate) cmd_rx: Receiver<ConnectionCommand>,
    pub(crate) sink_tx: SyncSender<super::sink::SinkChunk>,
    pub(crate) shared: Arc<Mutex<SharedState>>,
    pub(crate) event_tx: Option<Sender<ConnectionEvent>>,
    pub(crate) status: ConnectionStatus,
    pub(crate) generation: u64,
    pub(crate) target: Option<RfcommTarget>,
    pub(crate) socket: Option<Box<dyn ByteTransport>>,
    pub(crate) link_name: Option<String>,
    /// Coherent sink enabled+epoch (sole Owner state; snapshot in pump_linked).
    pub(crate) sink_fanout: SinkFanout,
    pub(crate) parser: RedDotStreamParser,
    pub(crate) next_enq: Instant,
    pub(crate) io_fail_streak: u32,
    pub(crate) connect_phase: ConnectPhase,
    pub(crate) connect_origin: ConnectOrigin,
    /// DIAGNOSE-ONLY: monotonic seq for successful socket reads with n > 0.
    pub(crate) diag_rx_seq: u64,
    /// DIAGNOSE-ONLY: last successful ENQ write Instant.
    pub(crate) diag_last_enq_sent_at: Option<Instant>,
    /// Set when Shutdown arrives while a Nuclear run pumps the command
    /// channel inline — `run()` must exit its loop afterwards.
    pub(crate) shutdown_requested: bool,
}

impl Owner {
    pub(crate) fn new(
        data_dir: PathBuf,
        cmd_rx: Receiver<ConnectionCommand>,
        sink_tx: SyncSender<super::sink::SinkChunk>,
        shared: Arc<Mutex<SharedState>>,
        event_tx: Option<Sender<ConnectionEvent>>,
    ) -> Self {
        let target = shared.lock().unwrap().target.clone();
        Self {
            data_dir,
            cmd_rx,
            sink_tx,
            shared,
            event_tx,
            status: ConnectionStatus::Idle,
            generation: 0,
            target,
            socket: None,
            link_name: None,
            sink_fanout: SinkFanout::default(),
            parser: RedDotStreamParser::new(),
            next_enq: Instant::now(),
            io_fail_streak: 0,
            connect_phase: ConnectPhase::Idle,
            connect_origin: ConnectOrigin::None,
            diag_rx_seq: 0,
            diag_last_enq_sent_at: None,
            shutdown_requested: false,
        }
    }

    pub(crate) fn run(mut self) {
        loop {
            let timeout = if self.status == ConnectionStatus::Linked {
                READ_SLICE
            } else {
                CMD_IDLE
            };

            match self.cmd_rx.recv_timeout(timeout) {
                Ok(cmd) => {
                    if self.handle_cmd(cmd) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            // Shutdown consumed inside run_nuclear's inline command pump.
            if self.shutdown_requested {
                break;
            }

            if self.status == ConnectionStatus::Linked {
                self.pump_linked();
            }
        }
        self.set_status(ConnectionStatus::ShuttingDown, "shutdown");
        self.socket = None;
    }

    pub(crate) fn handle_cmd(&mut self, cmd: ConnectionCommand) -> bool {
        match cmd {
            ConnectionCommand::Shutdown => {
                self.bump_generation();
                self.socket = None;
                spp_com::restore_all();
                true
            }
            ConnectionCommand::Start => {
                // Single-flight: never start a second Nuclear while one is Connecting.
                if self.status == ConnectionStatus::Connecting {
                    return false;
                }
                // Startup Nuclear only with persisted Known BD_ADDR — never name-scan/adopt.
                if self.target.is_none() {
                    self.set_status(
                        ConnectionStatus::NeedsTarget,
                        "Kein RedDot — „RedDot einrichten“",
                    );
                    return false;
                }
                self.after_known_ready();
                false
            }
            ConnectionCommand::PauseForSetup => {
                self.bump_generation();
                self.socket = None;
                self.link_name = None;
                self.set_status(ConnectionStatus::Discovering, "setup pause");
                false
            }
            ConnectionCommand::SelectTarget(t) => {
                self.bump_generation();
                self.socket = None;
                if let Err(e) = save_known_target(&self.data_dir, &t) {
                    self.set_status(ConnectionStatus::Faulted, &e);
                    return false;
                }
                self.target = Some(t);
                self.sync_shared_target();
                self.set_connect_origin(ConnectOrigin::None);
                self.set_connect_phase(ConnectPhase::Idle);
                self.set_status(ConnectionStatus::Idle, "Ziel gespeichert — Verbinden");
                false
            }
            ConnectionCommand::NuclearLink {
                bt_addr,
                display_name,
                origin,
            } => {
                if self.status == ConnectionStatus::Connecting {
                    // Single-flight: Badge/Setup attach via poll; do not Forget/Pair again.
                    return false;
                }
                self.run_nuclear(bt_addr, display_name, origin);
                false
            }
            ConnectionCommand::ForgetTarget => {
                self.bump_generation();
                self.socket = None;
                self.link_name = None;
                let addr = self.target.as_ref().map(|t| t.bt_addr);
                if let Some(addr) = addr {
                    let _ = crate::transport::rfcomm::discovery::remove_bond(addr);
                    spp_com::restore_for(addr);
                } else {
                    spp_com::restore_all();
                }
                let _ = clear_known_target(&self.data_dir);
                self.target = None;
                auth_hook::clear_auto_pin_allows();
                self.sync_shared_target();
                self.emit(
                    ConnectionEventKind::TargetCleared,
                    ConnectionStatus::NeedsTarget,
                    "forgotten",
                );
                self.set_status(ConnectionStatus::NeedsTarget, "forgotten");
                false
            }
            ConnectionCommand::RegisterSink => {
                // Epoch unchanged; Bridge captures shared.sink_epoch after this flips registered.
                self.sink_fanout = SinkFanout {
                    enabled: true,
                    epoch: self.sink_fanout.epoch,
                };
                let mut g = self.shared.lock().unwrap();
                g.sink_epoch = self.sink_fanout.epoch;
                g.sink_registered = true;
                false
            }
            ConnectionCommand::UnregisterSink => {
                let prev = self.sink_fanout.epoch;
                self.sink_fanout = SinkFanout {
                    enabled: false,
                    epoch: prev.wrapping_add(1),
                };
                self.parser = RedDotStreamParser::new();
                let mut g = self.shared.lock().unwrap();
                if let Some(rx) = g.sink_rx.as_mut() {
                    while rx.try_recv().is_ok() {}
                }
                g.sink_epoch = self.sink_fanout.epoch;
                g.sink_registered = false;
                false
            }
            ConnectionCommand::WriteBytes(data) => {
                if let Some(sock) = self.socket.as_mut() {
                    if let Err(e) = sock.write_all(&data, Duration::from_millis(500)) {
                        self.on_link_lost(&format!("write: {e}"));
                    }
                }
                false
            }
            ConnectionCommand::CancelConnect => {
                self.bump_generation();
                self.socket = None;
                self.link_name = None;
                self.auth_session_cleanup();
                self.set_connect_origin(ConnectOrigin::None);
                self.set_connect_phase(ConnectPhase::Idle);
                self.set_status(ConnectionStatus::Idle, "Abgebrochen — Verbinden");
                false
            }
        }
    }

    /// Known → exactly one Startup Nuclear. Bond-Gate is diag only (not Soft-A gate).
    /// Labs: `REDOT_SKIP_SOFT_AUTOCONNECT=1` → load Known, stay Idle (no Nuclear).
    fn after_known_ready(&mut self) {
        if std::env::var_os("REDOT_SKIP_SOFT_AUTOCONNECT").is_some() {
            self.set_connect_origin(ConnectOrigin::None);
            self.set_connect_phase(ConnectPhase::Idle);
            self.set_status(ConnectionStatus::Idle, "Nicht verbunden");
            return;
        }
        let Some(t) = self.target.clone() else {
            self.set_status(
                ConnectionStatus::NeedsTarget,
                "Kein RedDot — „RedDot einrichten“",
            );
            return;
        };
        // Bond lookup for logs only — does not choose Soft vs Nuclear.
        let bond = self.bond_lookup();
        self.diag(
            "startup_bond_diag",
            ConnectionStatus::Connecting,
            &format!("bond={bond:?}"),
            None,
            None,
        );
        self.run_nuclear(t.bt_addr, t.display_name, ConnectOrigin::StartupAuto);
    }

    fn auth_session_cleanup(&self) {
        auth_hook::clear_auto_pin_allows();
        auth_hook::uninstall_reddot_pin_hook();
        spp_com::restore_all();
    }

    fn run_nuclear(&mut self, bt_addr: u64, display_name: String, origin: ConnectOrigin) {
        if self.status == ConnectionStatus::Connecting {
            return;
        }
        self.bump_generation();
        self.socket = None;
        self.link_name = None;
        let gen0 = self.generation;
        let forget_scope = match origin {
            ConnectOrigin::StartupAuto => ForgetScope::PrimaryOnly,
            _ => ForgetScope::AllRedDotHints,
        };
        let addr_hex = format!("{:012X}", bt_addr & 0xFFFF_FFFF_FFFF);
        let run_id = diag::startup_run_id(gen0);
        self.set_connect_origin(origin);
        self.set_connect_phase(ConnectPhase::Paging);
        self.set_status(ConnectionStatus::Connecting, "Gerät wird vorbereitet…");

        let prev_addr = switch_forget_addr(
            self.target.as_ref().map(|t| t.bt_addr),
            bt_addr,
            forget_scope,
        );

        let shared = Arc::clone(&self.shared);
        let shared_prog = Arc::clone(&self.shared);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        thread::spawn(move || {
            if let Some(old) = prev_addr {
                let _ = crate::transport::rfcomm::discovery::remove_bond(old);
                spp_com::restore_for(old);
            }
            let result = nuclear::run_nuclear_link_with(
                bt_addr,
                &display_name,
                forget_scope,
                |phase| {
                    let mut g = shared_prog.lock().unwrap();
                    g.status = ConnectionStatus::Connecting;
                    g.last_reason = phase.to_string();
                    g.connect_phase = ConnectPhase::Paging;
                },
                || shared.lock().unwrap().generation != gen0,
            );
            let _ = tx.send(result);
        });

        // Pump cancel/shutdown while Nuclear runs on a worker.
        // NuclearLink / Start: drop (single-flight; callers attach via poll).
        let result = loop {
            match rx.try_recv() {
                Ok(r) => break r,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break Err(nuclear::NuclearFail {
                        message: "Nuclear Worker beendet".into(),
                        report: nuclear::NuclearRunReport {
                            forget_scope,
                            forget_result: "n/a".into(),
                            forget_duration_ms: 0,
                            pair_result: "n/a".into(),
                            pair_duration_ms: 0,
                            auth_hook_installed: false,
                            auth_callback_count: 0,
                            rfcomm_channel: None,
                            rfcomm_result: "n/a".into(),
                            failed_step: Some("worker"),
                            winsock: None,
                            cancelled: false,
                            duration_ms: 0,
                            hook_deregistered: false,
                        },
                    });
                }
            }
            match self.cmd_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(ConnectionCommand::CancelConnect)
                | Ok(ConnectionCommand::PauseForSetup)
                | Ok(ConnectionCommand::ForgetTarget) => {
                    self.bump_generation();
                }
                Ok(ConnectionCommand::Shutdown) => {
                    self.bump_generation();
                    self.socket = None;
                    spp_com::restore_all();
                    self.shutdown_requested = true;
                    let _ = rx.recv_timeout(Duration::from_secs(2));
                    return;
                }
                Ok(ConnectionCommand::NuclearLink { .. }) | Ok(ConnectionCommand::Start) => {}
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.bump_generation();
                    let _ = rx.recv_timeout(Duration::from_secs(1));
                    return;
                }
            }
        };

        if gen0 != self.generation {
            self.auth_session_cleanup();
            self.set_connect_origin(ConnectOrigin::None);
            self.set_connect_phase(ConnectPhase::Idle);
            if origin == ConnectOrigin::StartupAuto {
                let mut report = match result {
                    Ok((_, _, r)) => r,
                    Err(e) => e.report,
                };
                report.cancelled = true;
                report.hook_deregistered = true;
                report.failed_step = Some("cancelled");
                self.log_startup_nuclear(&run_id, &addr_hex, &report, false, "idle");
            }
            self.set_status(ConnectionStatus::Idle, "Abgebrochen — Verbinden");
            return;
        }

        match result {
            Ok((t, sock, report)) => {
                if let Err(e) = save_known_target(&self.data_dir, &t) {
                    self.set_status(ConnectionStatus::Faulted, &e);
                    return;
                }
                self.target = Some(t);
                self.sync_shared_target();
                let label = format!("RFCOMM ch={}", sock.channel.unwrap_or(1));
                self.socket = Some(Box::new(sock));
                self.link_name = Some(label.clone());
                self.parser = RedDotStreamParser::new();
                self.io_fail_streak = 0;
                self.next_enq = Instant::now() + POST_CONNECT_SETTLE;
                self.set_connect_phase(ConnectPhase::Idle);
                if origin == ConnectOrigin::StartupAuto {
                    self.log_startup_nuclear(&run_id, &addr_hex, &report, true, "linked");
                }
                self.diag(
                    "linked",
                    ConnectionStatus::Linked,
                    &format!("nuclear {label}"),
                    None,
                    None,
                );
                self.set_status(ConnectionStatus::Linked, &format!("connected ({label})"));
                self.emit(
                    ConnectionEventKind::Linked,
                    ConnectionStatus::Linked,
                    "connected",
                );
            }
            Err(e) if e.message == "Abgebrochen" => {
                self.auth_session_cleanup();
                self.set_connect_origin(ConnectOrigin::None);
                self.set_connect_phase(ConnectPhase::Idle);
                if origin == ConnectOrigin::StartupAuto {
                    self.log_startup_nuclear(&run_id, &addr_hex, &e.report, false, "idle");
                }
                self.set_status(ConnectionStatus::Idle, "Abgebrochen — Verbinden");
            }
            Err(e) => {
                let fail_status = if origin == ConnectOrigin::StartupAuto {
                    ConnectionStatus::Idle
                } else {
                    ConnectionStatus::NeedsPairing
                };
                self.set_connect_phase(ConnectPhase::AuthStop);
                if origin == ConnectOrigin::StartupAuto {
                    self.log_startup_nuclear(&run_id, &addr_hex, &e.report, false, "idle");
                }
                self.diag("nuclear_fail", fail_status, &e.message, None, e.report.winsock);
                self.set_connect_origin(ConnectOrigin::None);
                if origin == ConnectOrigin::StartupAuto {
                    self.set_status(ConnectionStatus::Idle, "Nicht verbunden");
                } else {
                    self.set_status(
                        ConnectionStatus::NeedsPairing,
                        &format!(
                            "Verbindung neu aufsetzen fehlgeschlagen — {}",
                            e.message
                        ),
                    );
                }
            }
        }
    }

    pub(crate) fn bond_lookup(&self) -> BondLookup {
        let Some(t) = self.target.as_ref() else {
            return BondLookup::NotBonded;
        };
        BondLookup::from_bond_result(bond_state(t.bt_addr))
    }

    pub(crate) fn set_connect_phase(&mut self, phase: ConnectPhase) {
        self.connect_phase = phase;
        self.shared.lock().unwrap().connect_phase = phase;
    }

    pub(crate) fn set_connect_origin(&mut self, origin: ConnectOrigin) {
        self.connect_origin = origin;
        self.shared.lock().unwrap().connect_origin = origin;
    }

    pub(crate) fn diag(
        &self,
        event: &str,
        status: ConnectionStatus,
        reason: &str,
        channel: Option<u32>,
        winsock: Option<i32>,
    ) {
        let addr = self.target.as_ref().map(|t| t.addr_hex());
        let ws_name = winsock.map(diag::winsock_name);
        diag::append(
            &self.data_dir,
            &DiagEvent {
                ts: diag::now_ts(),
                event,
                status: status.as_str(),
                reason,
                generation: Some(self.generation),
                addr: addr.as_deref(),
                channel,
                winsock,
                winsock_name: ws_name,
                attempt: None,
                silent: None,
                auth_hook_installed: None,
                release_attempted: None,
                result: None,
                toast_risk_path_entered: None,
            },
        );
    }

    fn log_startup_nuclear(
        &self,
        run_id: &str,
        addr_hex: &str,
        report: &nuclear::NuclearRunReport,
        linked: bool,
        next_state: &str,
    ) {
        let ws_name = report.winsock.map(diag::winsock_name);
        diag::append_startup_nuclear(
            &self.data_dir,
            &diag::StartupNuclearLog {
                ts: diag::now_ts(),
                event: "startup_nuclear",
                run_id,
                origin: "startupNuclear",
                generation: self.generation,
                target_bt_addr: addr_hex,
                forget_scope: report.forget_scope.as_api_str(),
                forget_result: &report.forget_result,
                forget_duration_ms: report.forget_duration_ms,
                pair_result: &report.pair_result,
                pair_duration_ms: report.pair_duration_ms,
                auth_hook_installed: report.auth_hook_installed,
                auth_callback_count: report.auth_callback_count,
                rfcomm_channel: report.rfcomm_channel,
                rfcomm_result: &report.rfcomm_result,
                linked,
                duration_ms: report.duration_ms,
                retry_scheduled: false,
                visible_toast_observed: None,
                failed_step: report.failed_step,
                winsock: report.winsock,
                winsock_name: ws_name,
                cancelled: report.cancelled,
                next_state,
                hook_deregistered: report.hook_deregistered,
            },
        );
    }

    pub(crate) fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.shared.lock().unwrap().generation = self.generation;
    }

    pub(crate) fn sync_shared_target(&self) {
        self.shared.lock().unwrap().target = self.target.clone();
    }

    pub(crate) fn set_status(&mut self, status: ConnectionStatus, reason: &str) {
        self.status = status;
        {
            let mut g = self.shared.lock().unwrap();
            g.status = status;
            g.last_reason = reason.to_string();
            g.generation = self.generation;
            g.target = self.target.clone();
            g.connect_phase = self.connect_phase;
            g.connect_origin = self.connect_origin;
        }
        self.emit(ConnectionEventKind::StatusChanged, status, reason);
    }

    pub(crate) fn emit(&self, kind: ConnectionEventKind, status: ConnectionStatus, reason: &str) {
        if let Some(tx) = &self.event_tx {
            let target = self.target.as_ref().map(|t| t.summary());
            let _ = tx.send(ConnectionEvent {
                kind,
                status,
                reason: reason.to_string(),
                generation: self.generation,
                target,
            });
        }
    }
}

/// Target switch (Badge/Setup): previous known bond to drop explicitly, so the
/// old device can never win the next paired-first scan. `AllRedDotHints` only
/// covers name-hint bonds and could miss a renamed old target. Startup
/// (`PrimaryOnly`) never switches — always `None` there.
pub(crate) fn switch_forget_addr(
    current: Option<u64>,
    new_addr: u64,
    scope: ForgetScope,
) -> Option<u64> {
    if scope != ForgetScope::AllRedDotHints {
        return None;
    }
    current
        .map(|a| a & 0xFFFF_FFFF_FFFF)
        .filter(|a| *a != new_addr & 0xFFFF_FFFF_FFFF)
}
