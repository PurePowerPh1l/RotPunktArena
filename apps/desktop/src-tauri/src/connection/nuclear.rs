//! Nuclear link: Forget → Pair → RFCOMM.
//!
//! Product: Startup (Known BD_ADDR, once) + Badge/Setup Verbinden.
//! No Soft-Wake. Lab twin: `bt_reset_connect`.

use crate::transport::rfcomm::auth_hook::{self, SetupAuthGuard};
use crate::transport::rfcomm::discovery::{
    bond_state, enumerate_paired, name_hint_rank, remove_bond, REDDOT_PAIR_PIN,
};
use crate::transport::rfcomm::socket::RfcommSocket;
use crate::transport::rfcomm::spp_com;
use crate::transport::rfcomm::target::{RfcommTarget, SPP_SERVICE_UUID};
use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant};

const FORGET_SETTLE: Duration = Duration::from_secs(2);
const PAIR_SETTLE: Duration = Duration::from_secs(1);
const PAGE: Duration = Duration::from_secs(12);
const RETRY_PAUSE: Duration = Duration::from_secs(2);
const PAIR_RETRIES: u32 = 3;
const RFCOMM_RETRIES: u32 = 3;

/// How many Windows bonds Nuclear may remove before Pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgetScope {
    /// Startup: **only** the persisted Known BD_ADDR — no name hints, no paired enum.
    PrimaryOnly,
    /// Badge / Setup repair: primary + every paired name-hint RedDot.
    AllRedDotHints,
}

impl ForgetScope {
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::PrimaryOnly => "primaryOnly",
            Self::AllRedDotHints => "allRedDotHints",
        }
    }
}

/// Step timings / results for soak JSONL (observation, not causal claims).
#[derive(Debug, Clone)]
pub struct NuclearRunReport {
    pub forget_scope: ForgetScope,
    pub forget_result: String,
    pub forget_duration_ms: u64,
    pub pair_result: String,
    pub pair_duration_ms: u64,
    pub auth_hook_installed: bool,
    pub auth_callback_count: u32,
    pub rfcomm_channel: Option<u32>,
    pub rfcomm_result: String,
    pub failed_step: Option<&'static str>,
    pub winsock: Option<i32>,
    pub cancelled: bool,
    pub duration_ms: u64,
    pub hook_deregistered: bool,
}

impl NuclearRunReport {
    fn new(forget_scope: ForgetScope) -> Self {
        Self {
            forget_scope,
            forget_result: "n/a".into(),
            forget_duration_ms: 0,
            pair_result: "n/a".into(),
            pair_duration_ms: 0,
            auth_hook_installed: false,
            auth_callback_count: 0,
            rfcomm_channel: None,
            rfcomm_result: "n/a".into(),
            failed_step: None,
            winsock: None,
            cancelled: false,
            duration_ms: 0,
            hook_deregistered: false,
        }
    }
}

#[derive(Debug)]
pub struct NuclearFail {
    pub message: String,
    pub report: NuclearRunReport,
}

fn wait_authenticated(addr: u64, deadline: Duration) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < deadline {
        if matches!(bond_state(addr), Ok(Some(b)) if b.authenticated) {
            return true;
        }
        thread::sleep(Duration::from_millis(200));
    }
    matches!(bond_state(addr), Ok(Some(b)) if b.authenticated)
}

/// Remove Windows bonds for `primary` and every paired name-hint RedDot.
pub fn forget_reddot_bonds(primary: u64) {
    forget_bonds(primary, ForgetScope::AllRedDotHints);
}

/// Forget bonds according to scope.
///
/// `PrimaryOnly`: `remove_bond(primary)` only — never `enumerate_paired`, never name hints.
fn forget_bonds(primary: u64, scope: ForgetScope) {
    let primary = primary & 0xFFFF_FFFF_FFFF;
    match scope {
        ForgetScope::PrimaryOnly => {
            let _ = remove_bond(primary);
            spp_com::restore_for(primary);
        }
        ForgetScope::AllRedDotHints => {
            let mut addrs: HashSet<u64> = HashSet::new();
            addrs.insert(primary);
            if let Ok(list) = enumerate_paired() {
                for d in list {
                    if name_hint_rank(&d.display_name).is_some() {
                        addrs.insert(d.bt_addr & 0xFFFF_FFFF_FFFF);
                    }
                }
            }
            for addr in addrs {
                let _ = remove_bond(addr);
                spp_com::restore_for(addr);
            }
            spp_com::restore_all();
        }
    }
}

/// Lab / simple call — no UI progress, not cancellable mid-flight.
pub fn run_nuclear_link(
    bt_addr: u64,
    display_name: &str,
) -> Result<(RfcommTarget, RfcommSocket), String> {
    match run_nuclear_link_with(
        bt_addr,
        display_name,
        ForgetScope::AllRedDotHints,
        |_| {},
        || false,
    ) {
        Ok((t, s, _)) => Ok((t, s)),
        Err(e) => Err(e.message),
    }
}

/// Forget → Pair → RFCOMM with phase labels, cooperative cancel, and soak report.
pub fn run_nuclear_link_with(
    bt_addr: u64,
    display_name: &str,
    forget_scope: ForgetScope,
    mut on_phase: impl FnMut(&str),
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<(RfcommTarget, RfcommSocket, NuclearRunReport), NuclearFail> {
    let wall = Instant::now();
    let mut report = NuclearRunReport::new(forget_scope);
    let addr = bt_addr & 0xFFFF_FFFF_FFFF;
    let mut name = display_name.trim().to_string();
    if name.is_empty() {
        name = format!("RedDot {addr:012X}");
    }

    let fail = |message: String, mut report: NuclearRunReport, step: &'static str| -> NuclearFail {
        report.failed_step = Some(step);
        report.duration_ms = wall.elapsed().as_millis() as u64;
        report.cancelled = message == "Abgebrochen";
        NuclearFail { message, report }
    };

    let check =
        |is_cancelled: &mut dyn FnMut() -> bool, report: &NuclearRunReport| -> Result<(), NuclearFail> {
            if is_cancelled() {
                Err(NuclearFail {
                    message: "Abgebrochen".into(),
                    report: {
                        let mut r = report.clone();
                        r.cancelled = true;
                        r.failed_step = Some("cancelled");
                        r.duration_ms = wall.elapsed().as_millis() as u64;
                        r
                    },
                })
            } else {
                Ok(())
            }
        };

    auth_hook::reset_auth_callback_count();

    on_phase("Gerät wird vorbereitet…");
    check(&mut is_cancelled, &report)?;
    let t_forget = Instant::now();
    forget_bonds(addr, forget_scope);
    report.forget_duration_ms = t_forget.elapsed().as_millis() as u64;
    report.forget_result = "ok".into();
    thread::sleep(FORGET_SETTLE);
    check(&mut is_cancelled, &report)?;

    on_phase("Kopple RedDot…");
    let t_pair = Instant::now();
    let mut paired = false;
    let mut last_pair_err = String::from("Pairing fehlgeschlagen");
    for i in 1..=PAIR_RETRIES {
        check(&mut is_cancelled, &report)?;
        match auth_hook::pair_with_pin_exclusive(addr, &name, REDDOT_PAIR_PIN) {
            Ok(()) => {
                paired = true;
                report.pair_result = "ok".into();
                break;
            }
            Err(e) => {
                last_pair_err = e.to_string();
                if matches!(bond_state(addr), Ok(Some(b)) if b.authenticated) {
                    paired = true;
                    report.pair_result = format!("ok(bondAuthenticatedAfterErr:{last_pair_err})");
                    break;
                }
                if i < PAIR_RETRIES {
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    }
    report.pair_duration_ms = t_pair.elapsed().as_millis() as u64;
    report.auth_callback_count = auth_hook::take_auth_callback_count();
    if !paired {
        report.pair_result = format!("fail:{last_pair_err}");
        return Err(fail(last_pair_err, report, "pair"));
    }
    if !wait_authenticated(addr, Duration::from_secs(15)) {
        report.pair_result = format!("{}|waitAuthTimeout", report.pair_result);
        return Err(fail(
            "Kopplung noch nicht fertig — erneut versuchen".into(),
            report,
            "pair",
        ));
    }
    check(&mut is_cancelled, &report)?;
    thread::sleep(PAIR_SETTLE);

    let _ = spp_com::release_channel_for(addr);

    let target = RfcommTarget {
        bt_addr: addr,
        display_name: name.clone(),
        service_uuid: SPP_SERVICE_UUID.to_string(),
        rfcomm_channel: Some(1),
        com_port: None,
    };

    on_phase(&format!("Verbinde mit {name}…"));
    let guard = match SetupAuthGuard::enter(addr) {
        Ok(g) => {
            report.auth_hook_installed = true;
            g
        }
        Err(e) => {
            report.auth_hook_installed = false;
            return Err(fail(e.to_string(), report, "hook"));
        }
    };
    let mut last_err = String::from("RFCOMM connect fehlgeschlagen");
    let mut last_ws: Option<i32> = None;
    for i in 1..=RFCOMM_RETRIES {
        check(&mut is_cancelled, &report)?;
        match RfcommSocket::connect(&target, PAGE) {
            Ok(sock) => {
                let ch = sock.channel.unwrap_or(1);
                report.rfcomm_channel = Some(ch);
                report.rfcomm_result = "ok".into();
                report.duration_ms = wall.elapsed().as_millis() as u64;
                drop(guard);
                report.hook_deregistered = true;
                report.auth_callback_count += auth_hook::take_auth_callback_count();
                return Ok((target, sock, report));
            }
            Err(e) => {
                last_ws = e.winsock_code();
                last_err = e.to_string();
                if i < RFCOMM_RETRIES {
                    thread::sleep(RETRY_PAUSE);
                }
            }
        }
    }
    drop(guard);
    report.hook_deregistered = true;
    report.auth_callback_count += auth_hook::take_auth_callback_count();
    report.rfcomm_result = format!("fail:{last_err}");
    report.winsock = last_ws;
    Err(fail(last_err, report, "rfcomm"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_only_api_str() {
        assert_eq!(ForgetScope::PrimaryOnly.as_api_str(), "primaryOnly");
    }
}
