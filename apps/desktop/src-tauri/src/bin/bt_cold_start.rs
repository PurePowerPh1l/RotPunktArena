//! Cold-start lab: observe Startup Nuclear / Idle / NeedsTarget after manager start.
//!
//!   cargo run --bin bt_cold_start --features rfcomm
//!
//! Note: product path is Startup Nuclear (not Soft-Wake). This bin watches for
//! stuck Connecting after the startup window.

use reddot_desktop_lib::connection::{ConnectionCommand, ConnectionManager, ConnectionStatus};
use reddot_desktop_lib::rfcomm::discovery::bond_state;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
}

fn main() {
    eprintln!("=== bt_cold_start (Startup Nuclear / Idle) ===");
    let dir = data_dir();
    eprintln!("data_dir={}", dir.display());

    let mgr = ConnectionManager::start(dir, None);
    let h = mgr.handle();

    let watch = Duration::from_secs(45);
    let t0 = Instant::now();
    let mut last = String::new();
    let mut saw_nuclear_reason = false;

    while t0.elapsed() < watch {
        let st = h.status();
        let reason = h.last_reason();
        let line = format!("{} | {reason}", st.as_str());
        if line != last {
            eprintln!("  [{:.1?}] {line}", t0.elapsed());
            last = line;
        }
        if reason.contains("Kopplung zurücksetzen") {
            saw_nuclear_reason = true;
        }
        if matches!(
            st,
            ConnectionStatus::Linked
                | ConnectionStatus::Idle
                | ConnectionStatus::NeedsTarget
                | ConnectionStatus::NeedsPairing
                | ConnectionStatus::Faulted
        ) && t0.elapsed() > Duration::from_secs(1)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let st = h.status();
    let has_target = h.target().is_some();
    let bonded = h
        .target()
        .as_ref()
        .and_then(|t| bond_state(t.bt_addr).ok())
        .flatten()
        .map(|b| b.authenticated)
        .unwrap_or(false);

    eprintln!(
        "final: status={} target={} bonded={}",
        st.as_str(),
        has_target,
        bonded
    );

    let _ = h.send(ConnectionCommand::Shutdown);
    std::thread::sleep(Duration::from_millis(200));

    if st == ConnectionStatus::Connecting {
        eprintln!("FAIL: noch Connecting nach Watch-Fenster — Loop?");
        std::process::exit(3);
    }

    if st == ConnectionStatus::Linked {
        if !bonded && !has_target {
            eprintln!("FAIL Linked ohne Target");
            std::process::exit(4);
        }
        eprintln!("PASS Linked (startup nuclear={saw_nuclear_reason})");
        return;
    }

    if has_target {
        if !matches!(
            st,
            ConnectionStatus::Idle | ConnectionStatus::NeedsPairing | ConnectionStatus::Faulted
        ) {
            eprintln!("FAIL: mit Known erwartet Idle/Linked, got {}", st.as_str());
            std::process::exit(4);
        }
        eprintln!("PASS Known, Idle/NeedsPairing — tippe Verbinden");
    } else {
        if st != ConnectionStatus::NeedsTarget && st != ConnectionStatus::Idle {
            eprintln!(
                "WARN: ohne Known erwartet needsTarget, got {}",
                st.as_str()
            );
        }
        eprintln!("PASS kein Known / NeedsTarget — Sheet-Fall");
    }
}
