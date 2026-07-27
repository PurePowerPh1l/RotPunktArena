//! Nuclear twice: Forget→Pair→RFCOMM, drop, stiller Pause, nochmal Nuclear.
//!
//!   cargo run --bin bt_nuclear_twice --features rfcomm
//!
//! Simulates: Verbinden → Disconnect → Verbinden again.

use reddot_desktop_lib::connection::run_nuclear_link;
use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_reddot_candidate},
    target::RfcommTarget, WinsockRuntime,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn load_known() -> Option<RfcommTarget> {
    let path = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
        .join("rfcomm_known_target.json");
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn resolve() -> RfcommTarget {
    load_known()
        .or_else(|| find_reddot_candidate().ok().flatten())
        .map(|mut t| {
            t.rfcomm_channel = Some(1);
            t
        })
        .expect("Kein Known / Candidate")
}

fn print_bond(addr: u64, label: &str) {
    match bond_state(addr) {
        Ok(Some(b)) => eprintln!(
            "  bond {label}: auth={} connected={}",
            b.authenticated, b.connected
        ),
        _ => eprintln!("  bond {label}: unknown"),
    }
}

fn main() {
    eprintln!("=== bt_nuclear_twice ===");
    let _ = WinsockRuntime::init();
    let t = resolve();
    eprintln!("Target {} @ {:012X}", t.display_name, t.bt_addr);

    for round in 1..=2 {
        eprintln!("\n--- round {round} nuclear ---");
        print_bond(t.bt_addr, "before");
        let t0 = Instant::now();
        match run_nuclear_link(t.bt_addr, &t.display_name) {
            Ok((tgt, sock)) => {
                eprintln!(
                    "  OK {} in {:.0?}",
                    tgt.display_name,
                    t0.elapsed()
                );
                print_bond(t.bt_addr, "linked");
                std::thread::sleep(Duration::from_secs(2));
                drop(sock);
                eprintln!("  dropped socket");
            }
            Err(e) => {
                eprintln!("FAIL round {round}: {e}");
                std::process::exit(2);
            }
        }
        if round == 1 {
            eprintln!("  gap 3s before second Verbinden…");
            std::thread::sleep(Duration::from_secs(3));
        }
    }
    print_bond(t.bt_addr, "after");
    eprintln!("PASS two nuclear cycles");
}
