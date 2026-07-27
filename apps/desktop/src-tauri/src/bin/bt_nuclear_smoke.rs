//! Nuclear Verbinden via ConnectionManager — **ohne** Soft-Autoconnect.
//!
//!   cargo run --bin bt_nuclear_smoke --features rfcomm
//!
//! Sets `REDOT_SKIP_SOFT_AUTOCONNECT=1` so Start only loads Known/Idle (no Startup Nuclear).
//! Then Nuclear only: Forget → Pair → RFCOMM (Auth vor AF_BTH, kein Soft-Hook davor).

use reddot_desktop_lib::connection::{
    connect_known_nuclear, needs_setup, ConnectionCommand, ConnectionManager, ConnectionStatus,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const HOLD: Duration = Duration::from_secs(20);

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.reddot.arena")
}

fn main() {
    eprintln!("=== bt_nuclear_smoke (Manager Nuclear only, no Soft) ===");
    std::env::set_var("REDOT_SKIP_SOFT_AUTOCONNECT", "1");

    let dir = data_dir();
    let mgr = ConnectionManager::start(dir, None);
    let h = mgr.handle();

    let settle0 = Instant::now();
    while settle0.elapsed() < Duration::from_secs(5) {
        let st = h.status();
        if matches!(
            st,
            ConnectionStatus::Idle
                | ConnectionStatus::NeedsTarget
                | ConnectionStatus::NeedsPairing
                | ConnectionStatus::Faulted
                | ConnectionStatus::Linked
        ) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if h.status() == ConnectionStatus::Linked {
        eprintln!("FAIL Soft/Auto Linked trotz REDOT_SKIP_SOFT_AUTOCONNECT");
        let _ = h.send(ConnectionCommand::Shutdown);
        std::process::exit(5);
    }

    if needs_setup(&h) || h.target().is_none() {
        eprintln!("FAIL needs_setup / kein Known — zuerst Setup-Sheet / bt_reset_connect");
        let _ = h.send(ConnectionCommand::Shutdown);
        std::process::exit(4);
    }

    eprintln!(
        "target={} status={} (erwartet Idle, dann Nuclear)",
        h.target()
            .map(|t| format!("{} @ {}", t.display_name, t.addr_hex()))
            .unwrap_or_else(|| "-".into()),
        h.status().as_str()
    );
    eprintln!("Nuclear connect_known_nuclear…");
    let t0 = Instant::now();
    match connect_known_nuclear(&h) {
        Ok(t) => eprintln!(
            "Linked {} @ {} in {:.0?}",
            t.display_name,
            t.addr_hex(),
            t0.elapsed()
        ),
        Err(e) => {
            eprintln!("FAIL nuclear: {e}");
            let _ = h.send(ConnectionCommand::Shutdown);
            std::process::exit(2);
        }
    }

    if h.status() != ConnectionStatus::Linked {
        eprintln!("FAIL status nach nuclear: {}", h.status().as_str());
        std::process::exit(2);
    }

    eprintln!("holding Linked {HOLD:?}…");
    let hold0 = Instant::now();
    while hold0.elapsed() < HOLD {
        if h.status() != ConnectionStatus::Linked {
            eprintln!(
                "FAIL link dropped after {:.0?} ({})",
                hold0.elapsed(),
                h.status().as_str()
            );
            let _ = h.send(ConnectionCommand::Shutdown);
            std::process::exit(3);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!("PASS held Linked {HOLD:?}");
    let _ = h.send(ConnectionCommand::Shutdown);
    std::thread::sleep(Duration::from_millis(300));
}
