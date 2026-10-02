//! First-setup scan + nuclear connect (blocking helpers for Tauri commands).
//!
//! Start: Startup Nuclear once for Known BD_ADDR.
//! Sheet / Badge Verbinden → NuclearLink on owner (single-flight with Startup).

use super::command::ConnectionCommand;
use super::connect_policy::ConnectOrigin;
use super::handle::ConnectionHandle;
use super::status::ConnectionStatus;
use crate::transport::rfcomm::discovery::{bond_state, find_reddot_candidate, scan_all_reddots};
use crate::transport::rfcomm::target::RfcommTarget;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupCandidate {
    pub bt_addr_hex: String,
    pub display_name: String,
    pub already_paired: bool,
    /// True for the currently persisted (active) device.
    pub is_active: bool,
}

fn bond_authenticated(bt_addr: u64) -> bool {
    matches!(bond_state(bt_addr), Ok(Some(b)) if b.authenticated)
}

/// Sheet only when there is no known target (and not already Linked).
pub fn needs_setup(handle: &ConnectionHandle) -> bool {
    if handle.status() == ConnectionStatus::Linked {
        return false;
    }
    if handle.target().is_some() {
        return false;
    }
    true
}

fn publish_setup(
    handle: &ConnectionHandle,
    generation: u64,
    status: ConnectionStatus,
    reason: &str,
) -> Result<(), String> {
    handle.send(ConnectionCommand::SetupProgress {
        generation,
        status,
        reason: reason.into(),
    })
}

fn pause_owner_for_setup(handle: &ConnectionHandle) -> Result<u64, String> {
    let previous = handle.snapshot().generation;
    handle.send(ConnectionCommand::PauseForSetup)?;
    // An in-flight Nuclear may still be inside a blocking Windows pairing or
    // socket call. The Owner acknowledges Discovering only after that worker
    // has stopped, so scanning cannot race its Bluetooth operations.
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        let snapshot = handle.snapshot();
        if snapshot.generation > previous && snapshot.status == ConnectionStatus::Discovering {
            return Ok(snapshot.generation);
        }
        thread::sleep(Duration::from_millis(40));
    }
    Err("Owner PauseForSetup nicht bestätigt".into())
}

/// Pause connect + scan for **all** RedDots (paired list + nearby inquiry).
///
/// Always runs both sources so a new/second device shows up even while an
/// old one is still bonded. Active (persisted) device sorts first.
pub fn setup_scan(handle: &ConnectionHandle) -> Result<Vec<SetupCandidate>, String> {
    let generation = pause_owner_for_setup(handle)?;
    publish_setup(
        handle,
        generation,
        ConnectionStatus::Discovering,
        "Suche RedDot in der Nähe…",
    )?;

    let active_addr = handle.target().map(|t| t.bt_addr & 0xFFFF_FFFF_FFFF);
    let devices = match scan_all_reddots() {
        Ok(devices) => devices,
        Err(error) => {
            publish_setup(
                handle,
                generation,
                ConnectionStatus::NeedsTarget,
                &error.to_string(),
            )?;
            return Err(error.to_string());
        }
    };
    if handle.generation() != generation {
        return Err("Gerätesuche abgebrochen".into());
    }
    if devices.is_empty() {
        let msg = "Kein RedDot gefunden — Ziel einschalten, nah ans Gerät halten, erneut suchen"
            .to_string();
        publish_setup(handle, generation, ConnectionStatus::NeedsTarget, &msg)?;
        return Err(msg);
    }

    let mut candidates: Vec<SetupCandidate> = devices
        .into_iter()
        .map(|d| {
            let addr = d.bt_addr & 0xFFFF_FFFF_FFFF;
            SetupCandidate {
                bt_addr_hex: format!("{addr:012X}"),
                display_name: d.display_name,
                already_paired: d.paired || bond_authenticated(addr),
                is_active: active_addr == Some(addr),
            }
        })
        .collect();
    // Active device first, discovery rank otherwise (stable sort).
    candidates.sort_by_key(|c| !c.is_active);

    let reason = if candidates.len() == 1 {
        format!("Gefunden: {}", candidates[0].display_name)
    } else {
        format!("{} RedDots gefunden", candidates.len())
    };
    publish_setup(handle, generation, ConnectionStatus::Discovering, &reason)?;
    Ok(candidates)
}

fn parse_bt_addr_hex(hex: &str) -> Result<u64, String> {
    let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() != 12 {
        return Err(format!("Ungültige Bluetooth-Adresse: {hex}"));
    }
    u64::from_str_radix(&clean, 16).map_err(|e| e.to_string())
}

/// Wait for an in-flight or just-sent Nuclear to settle (single-flight attach).
fn wait_nuclear_outcome(
    handle: &ConnectionHandle,
    expected_addr: u64,
    gen0: u64,
    attach: bool,
) -> Result<RfcommTarget, String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        let snapshot = handle.snapshot();
        let st = snapshot.status;
        let gen = snapshot.generation;
        if st == ConnectionStatus::Linked {
            // New Nuclear bumps gen; attach to Startup keeps the same gen on success.
            if attach || gen > gen0 {
                let target = snapshot
                    .target
                    .ok_or("Verbindung ohne aktives RedDot-Ziel")?;
                if target.bt_addr & 0xFFFF_FFFF_FFFF != expected_addr & 0xFFFF_FFFF_FFFF {
                    return Err("Ein anderes RedDot wurde verbunden — bitte erneut wählen".into());
                }
                return Ok(target);
            }
        }
        if st == ConnectionStatus::Connecting {
            thread::sleep(Duration::from_millis(200));
            continue;
        }
        // Flight ended without link.
        if (attach || gen > gen0)
            && matches!(
                st,
                ConnectionStatus::Faulted
                    | ConnectionStatus::NeedsPairing
                    | ConnectionStatus::NeedsTarget
                    | ConnectionStatus::Idle
            )
        {
            let reason = snapshot.reason;
            if st == ConnectionStatus::Idle
                && !attach
                && !reason.contains("Abgebrochen")
                && !reason.contains("fehlgeschlagen")
                && !reason.contains("Nicht verbunden")
            {
                thread::sleep(Duration::from_millis(200));
                continue;
            }
            return Err(if reason.is_empty() {
                "Verbindung fehlgeschlagen".into()
            } else {
                reason
            });
        }
        thread::sleep(Duration::from_millis(200));
    }
    let msg = handle.last_reason();
    Err(if msg.is_empty() {
        "Zeitüberschreitung beim Verbinden".into()
    } else {
        msg
    })
}

/// Nuclear link: Forget → Pair → RFCOMM (same as Verbinden button).
pub fn setup_connect(
    handle: &ConnectionHandle,
    bt_addr_hex: &str,
    display_name_hint: Option<&str>,
) -> Result<RfcommTarget, String> {
    let addr = parse_bt_addr_hex(bt_addr_hex)?;
    // Attach only to a known-device flight for this exact address. A setup
    // flight may target a different address while the old target remains active.
    if handle.status() == ConnectionStatus::Connecting
        && matches!(
            handle.connect_origin(),
            ConnectOrigin::StartupAuto | ConnectOrigin::BadgeNuclear
        )
        && handle.target().map(|t| t.bt_addr) == Some(addr)
    {
        return wait_nuclear_outcome(handle, addr, handle.generation(), true);
    }

    pause_owner_for_setup(handle)?;

    let mut display_name = display_name_hint
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("RedDot")
        .to_string();
    if display_name == "RedDot" {
        display_name = format!("RedDot {bt_addr_hex}");
    }
    if let Ok(Some(t)) = find_reddot_candidate() {
        if t.bt_addr == addr {
            display_name = t.display_name;
        }
    }

    let gen0 = handle.generation();
    handle.send(ConnectionCommand::NuclearLink {
        bt_addr: addr,
        display_name: display_name.clone(),
        origin: ConnectOrigin::SetupNuclear,
    })?;

    wait_nuclear_outcome(handle, addr, gen0, false)
}

/// Nuclear link for a known target (Verbinden button).
///
/// If Startup Nuclear is already `Connecting`, attaches to that flight — never a
/// second Forget/Pair sequence.
pub fn connect_known_nuclear(handle: &ConnectionHandle) -> Result<RfcommTarget, String> {
    let Some(t) = handle.target() else {
        return Err("Kein Ziel — „RedDot einrichten“".into());
    };

    if handle.status() == ConnectionStatus::Connecting {
        return wait_nuclear_outcome(handle, t.bt_addr, handle.generation(), true);
    }

    // Capture generation *before* send — Nuclear bumps generation on start.
    let gen0 = handle.generation();
    handle.send(ConnectionCommand::NuclearLink {
        bt_addr: t.bt_addr,
        display_name: t.display_name.clone(),
        origin: ConnectOrigin::BadgeNuclear,
    })?;

    wait_nuclear_outcome(handle, t.bt_addr, gen0, false)
}

pub fn open_windows_bluetooth_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        use ::windows::core::PCWSTR;
        use ::windows::Win32::UI::Shell::ShellExecuteW;
        use ::windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let url: Vec<u16> = "ms-settings:bluetooth\0".encode_utf16().collect();
        unsafe {
            let rc = ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if (rc.0 as isize) <= 32 {
                return Err(format!("ShellExecuteW failed ({})", rc.0 as isize));
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Bluetooth-Einstellungen nur unter Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::connect_policy::ConnectPhase;
    use crate::connection::shared::SharedState;
    use std::sync::{mpsc, Arc, Mutex};

    fn linked_handle(addr: u64) -> ConnectionHandle {
        let (cmd_tx, _cmd_rx) = mpsc::channel();
        ConnectionHandle {
            cmd_tx,
            inner: Arc::new(Mutex::new(SharedState {
                status: ConnectionStatus::Linked,
                generation: 2,
                target: Some(RfcommTarget {
                    bt_addr: addr,
                    display_name: "RedDot".into(),
                    service_uuid: crate::transport::rfcomm::SPP_SERVICE_UUID.into(),
                    rfcomm_channel: Some(1),
                    com_port: None,
                }),
                last_reason: String::new(),
                connect_phase: ConnectPhase::Idle,
                connect_origin: ConnectOrigin::None,
                sink_rx: None,
                sink_registered: false,
                sink_epoch: 0,
            })),
        }
    }

    #[test]
    fn linked_outcome_requires_requested_device() {
        let a = 0xAAAA_AAAA_AAAA;
        let b = 0xBBBB_BBBB_BBBB;
        let handle = linked_handle(a);
        assert_eq!(
            wait_nuclear_outcome(&handle, a, 1, false).unwrap().bt_addr,
            a
        );
        assert!(wait_nuclear_outcome(&handle, b, 1, false).is_err());
        assert!(wait_nuclear_outcome(&handle, b, 2, true).is_err());
    }
}
