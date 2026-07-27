//! RFCOMM soak aligned with product: warm → hold → reconnect.
//!   cargo run --bin bt_soak --features rfcomm
//!
//! AuthEx answers PIN `0000`. If a Windows dialog appears: **do nothing**
//! (don't type, don't cancel — both race the hook; cancel → send_rc≠0).

use reddot_desktop_lib::rfcomm::{
    auth_hook,
    discovery::{bond_state, find_reddot_candidate},
    target::RfcommTarget, ByteTransport, RfcommSocket, WinsockRuntime, SPP_SERVICE_UUID,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const POST_CONNECT_SETTLE: Duration = Duration::from_millis(800);
const ENQ_INTERVAL: Duration = Duration::from_millis(500);
const ENQ_WRITE_TIMEOUT: Duration = Duration::from_millis(3000);
const READ_SLICE: Duration = Duration::from_millis(50);
const ENQ_FAIL_ABORT: u32 = 6;
const HOLD_ENQ_TARGET: usize = 40;
const RECONNECT_N: usize = 15;
/// Deep-sleep paging needs wall time; 90s dies mid-soft-wake with 12s pages.
const WARM_DEADLINE: Duration = Duration::from_secs(180);

fn connect_timeout_for_attempt(attempt: u32, _last_err: Option<&str>) -> Duration {
    if attempt <= 10 {
        Duration::from_secs(12)
    } else {
        Duration::from_secs(45)
    }
}

fn delay_after_fail(_err: &str) -> Duration {
    Duration::from_secs(3)
}

fn load_known() -> Option<RfcommTarget> {
    let path = dirs_fallback().join("rfcomm_known_target.json");
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn dirs_fallback() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
}

fn connect_once(target: &RfcommTarget, timeout: Duration) -> Result<RfcommSocket, String> {
    RfcommSocket::connect(target, timeout).map_err(|e| e.to_string())
}

fn drain_auth_notes() {
    while let Some(n) = auth_hook::take_last_auth_note() {
        eprintln!("  [authHook] {n}");
        if n.contains("send_rc=0") {
            eprintln!(
                "  [authHook] PIN auto — Dialog nicht anfassen (kein Tippen, kein Abbrechen)"
            );
        } else if n.contains("send_rc=") {
            eprintln!(
                "  [authHook] Antwort fehlgeschlagen — Dialog evtl. abgebrochen/geraced"
            );
        }
    }
}

fn pump_reads(sock: &mut RfcommSocket, until: Instant) {
    let mut buf = [0u8; 256];
    while Instant::now() < until {
        match sock.read(&mut buf, READ_SLICE) {
            Ok(_) | Err(_) => {}
        }
    }
}

fn enq_write(sock: &mut RfcommSocket) -> Result<(), String> {
    sock.write_all(&[0x05], ENQ_WRITE_TIMEOUT)
        .map_err(|e| e.to_string())
}

/// Connect, settle, one ENQ — product-like link proof.
fn connect_verified(
    target: &RfcommTarget,
    timeout: Duration,
) -> Result<RfcommSocket, String> {
    let mut sock = connect_once(target, timeout)?;
    drain_auth_notes();
    std::thread::sleep(POST_CONNECT_SETTLE);
    enq_write(&mut sock)?;
    let mut buf = [0u8; 64];
    let _ = sock.read(&mut buf, Duration::from_millis(300));
    Ok(sock)
}

fn warm_until_linked(target: &RfcommTarget) -> Option<RfcommSocket> {
    let warm_secs = std::env::var("REDDOT_SOAK_WARM_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(WARM_DEADLINE.as_secs());
    eprintln!("Phase A: warm-up until first OK (max {warm_secs}s)…");
    let t0 = Instant::now();
    let deadline = t0 + Duration::from_secs(warm_secs);
    let mut attempt = 0u32;
    let mut last_err: Option<String> = None;
    let mut saw_auth = false;
    while Instant::now() < deadline {
        attempt += 1;
        let timeout = connect_timeout_for_attempt(attempt, last_err.as_deref());
        let left = deadline.saturating_duration_since(Instant::now());
        eprintln!(
            "  warm try {attempt} (timeout {}s, {left:.0?} left)…",
            timeout.as_secs()
        );
        match connect_verified(target, timeout) {
            Ok(sock) => {
                eprintln!(
                    "  warm OK on attempt {attempt} (connect+ENQ, {:.0?})",
                    t0.elapsed()
                );
                return Some(sock);
            }
            Err(e) => {
                let before = auth_hook::take_last_auth_note();
                if let Some(n) = before {
                    saw_auth = true;
                    eprintln!("  [authHook] {n}");
                    if n.contains("send_rc=0") {
                        eprintln!(
                            "  [authHook] PIN auto — Dialog nicht anfassen (kein Tippen, kein Abbrechen)"
                        );
                    }
                }
                // drain any extras
                drain_auth_notes();
                eprintln!("  warm try {attempt}: {e} ({:.0?} elapsed)", t0.elapsed());
                let d = delay_after_fail(&e);
                last_err = Some(e);
                std::thread::sleep(d);
            }
        }
    }
    eprintln!(
        "  warm gave up after {attempt} tries / {:.0?} (authChallenge={saw_auth})",
        t0.elapsed()
    );
    if !saw_auth {
        eprintln!(
            "  hint: kein AuthEx — Ziel tief schlafend/aus, oder BT-Stack hängt."
        );
        eprintln!(
            "  hint: Ziel kurz AUS/AN, ggf. Windows-Bluetooth togglen, dann Soak erneut."
        );
    }
    None
}

fn hold_enq_loop(target: &RfcommTarget, sock: RfcommSocket) -> usize {
    eprintln!(
        "Phase B: hold + {HOLD_ENQ_TARGET}× ENQ (1× reclaim on link loss)…"
    );
    let mut sock = Some(sock);
    let mut enq_ok = 0usize;
    let mut fail_streak = 0u32;
    let mut reclaimed = false;
    let mut next_enq = Instant::now();

    while enq_ok < HOLD_ENQ_TARGET {
        let Some(s) = sock.as_mut() else {
            break;
        };
        pump_reads(s, next_enq);
        next_enq = Instant::now() + ENQ_INTERVAL;

        match enq_write(s) {
            Ok(()) => {
                enq_ok += 1;
                fail_streak = 0;
                let mut buf = [0u8; 64];
                let _ = s.read(&mut buf, Duration::from_millis(200));
            }
            Err(e) => {
                fail_streak += 1;
                eprintln!("  ENQ fail at ok={enq_ok}: {e}");
                if fail_streak < ENQ_FAIL_ABORT {
                    continue;
                }
                if reclaimed {
                    eprintln!("  link lost again — stop hold");
                    break;
                }
                eprintln!("  link lost — reclaim once…");
                sock = None;
                std::thread::sleep(Duration::from_secs(2));
                match reclaim_socket(target) {
                    Ok(s) => {
                        sock = Some(s);
                        fail_streak = 0;
                        reclaimed = true;
                        next_enq = Instant::now();
                    }
                    Err(e) => {
                        eprintln!("  reclaim failed: {e}");
                        break;
                    }
                }
            }
        }
    }
    drop(sock);
    eprintln!("  ENQ ok {enq_ok}/{HOLD_ENQ_TARGET}");
    enq_ok
}

fn reclaim_socket(target: &RfcommTarget) -> Result<RfcommSocket, String> {
    let mut last_err: Option<String> = None;
    for attempt in 1u32..=8 {
        let timeout = connect_timeout_for_attempt(attempt, last_err.as_deref());
        match connect_verified(target, timeout) {
            Ok(s) => return Ok(s),
            Err(e) => {
                drain_auth_notes();
                let d = delay_after_fail(&e);
                last_err = Some(e);
                std::thread::sleep(d);
            }
        }
    }
    Err("reclaim exhausted".into())
}

fn reconnect_phase(target: &RfcommTarget) -> usize {
    eprintln!("Phase C: {RECONNECT_N}× reconnect + ENQ…");
    let mut ok = 0usize;
    let mut last_err: Option<String> = None;
    for i in 1..=RECONNECT_N {
        let t0 = Instant::now();
        let timeout = connect_timeout_for_attempt(i as u32, last_err.as_deref());
        match connect_verified(target, timeout) {
            Ok(s) => {
                drop(s);
                ok += 1;
                last_err = None;
                eprintln!(
                    "  [{i:02}/{RECONNECT_N}] OK connect+ENQ ({:.1}s)",
                    t0.elapsed().as_secs_f32()
                );
                std::thread::sleep(Duration::from_secs(2));
            }
            Err(e) => {
                drain_auth_notes();
                eprintln!(
                    "  [{i:02}/{RECONNECT_N}] FAIL {e} ({:.1}s)",
                    t0.elapsed().as_secs_f32()
                );
                let d = delay_after_fail(&e);
                last_err = Some(e);
                std::thread::sleep(d);
            }
        }
    }
    ok
}

fn main() {
    eprintln!("=== bt_soak (warm + hold + reconnect) ===");
    eprintln!(
        "MANUAL: Ziel AN. PIN-Dialog nicht tippen/abbrechen wenn AuthEx schon antwortet."
    );
    let _ = WinsockRuntime::init();
    if let Err(e) = auth_hook::install_reddot_pin_hook() {
        eprintln!("WARN PIN-Hook: {e}");
    }

    let mut target = load_known().unwrap_or_else(|| {
        find_reddot_candidate()
            .ok()
            .flatten()
            .expect("Kein Known-Target")
    });
    target.rfcomm_channel = Some(target.rfcomm_channel.unwrap_or(1));
    auth_hook::allow_auto_pin_for(target.bt_addr);

    match bond_state(target.bt_addr) {
        Ok(Some(b)) if b.authenticated => {
            eprintln!(
                "Bond OK (remembered={} connected={})",
                b.remembered, b.connected
            );
        }
        Ok(Some(b)) => {
            eprintln!("FAIL Bond nicht authentifiziert (remembered={})", b.remembered);
            std::process::exit(4);
        }
        Ok(None) => {
            eprintln!("FAIL Gerät unbekannt — App-Setup zuerst");
            std::process::exit(4);
        }
        Err(e) => {
            eprintln!("FAIL bond_state: {e}");
            std::process::exit(4);
        }
    }

    eprintln!(
        "Target {} @ {:012X} ch={:?}",
        target.display_name, target.bt_addr, target.rfcomm_channel
    );

    let Some(warm) = warm_until_linked(&target) else {
        eprintln!("FAIL warm-up");
        std::process::exit(2);
    };

    let enq_ok = hold_enq_loop(&target, warm);
    std::thread::sleep(Duration::from_secs(2));
    let recon_ok = reconnect_phase(&target);

    let recon_rate = recon_ok as f64 / RECONNECT_N as f64;
    let enq_rate = enq_ok as f64 / HOLD_ENQ_TARGET as f64;
    eprintln!(
        "=== reconnect+ENQ {recon_ok}/{RECONNECT_N} ({:.0}%), hold-ENQ {:.0}% ({enq_ok}/{HOLD_ENQ_TARGET}) ===",
        recon_rate * 100.0,
        enq_rate * 100.0
    );
    if recon_rate >= 0.8 && enq_rate >= 0.8 {
        eprintln!("PASS");
        std::process::exit(0);
    }
    if recon_rate >= 0.8 && enq_rate >= 0.5 {
        eprintln!("SOFT-FAIL: reconnect OK, hold weak. Check hold.");
        std::process::exit(1);
    }
    eprintln!("FAIL");
    std::process::exit(1);
}

#[allow(dead_code)]
fn _spp() -> &'static str {
    SPP_SERVICE_UUID
}
