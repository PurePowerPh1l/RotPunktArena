//! Product-path smoke: cold (Bond-Gate Soft oder Idle) → ggf. Nuclear → hold Linked.
//!   cargo run --bin bt_product_smoke --features rfcomm

use reddot_desktop_lib::connection::{
    connect_known_nuclear, needs_setup, ConnectionCommand, ConnectionManager, ConnectionStatus,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const HOLD: Duration = Duration::from_secs(45);

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
}

fn wait_terminal(h: &reddot_desktop_lib::connection::ConnectionHandle, max: Duration) {
    let t0 = Instant::now();
    while t0.elapsed() < max {
        let st = h.status();
        if matches!(
            st,
            ConnectionStatus::Linked
                | ConnectionStatus::Idle
                | ConnectionStatus::NeedsTarget
                | ConnectionStatus::NeedsPairing
                | ConnectionStatus::Faulted
        ) {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn main() {
    eprintln!("=== bt_product_smoke (cold Soft|Idle → nuclear? → hold) ===");
    let dir = data_dir();
    let mgr = ConnectionManager::start(dir, None);
    let h = mgr.handle();
    wait_terminal(&h, Duration::from_secs(45));

    let st = h.status();
    eprintln!("cold status={} reason={}", st.as_str(), h.last_reason());

    if needs_setup(&h) || h.target().is_none() {
        eprintln!("FAIL needs_setup — Setup-Sheet zuerst");
        std::process::exit(4);
    }

    if st != ConnectionStatus::Linked {
        eprintln!("nuclear (Soft nicht Linked)…");
        let t0 = Instant::now();
        if let Err(e) = connect_known_nuclear(&h) {
            eprintln!("FAIL nuclear: {e}");
            std::process::exit(3);
        }
        eprintln!("Linked in {:.0?}", t0.elapsed());
    } else {
        eprintln!("cold Soft bereits Linked — Nuclear übersprungen");
    }

    let hold0 = Instant::now();
    while hold0.elapsed() < HOLD {
        if h.status() != ConnectionStatus::Linked {
            eprintln!("FAIL drop during hold ({})", h.status().as_str());
            std::process::exit(2);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!("PASS held {HOLD:?}");
    let _ = h.send(ConnectionCommand::Shutdown);
    std::thread::sleep(Duration::from_millis(300));
}
