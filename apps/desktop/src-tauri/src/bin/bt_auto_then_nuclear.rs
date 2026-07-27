//! Autoconnect once → on fail Nuclear (geplante Produkt-Staffelung).
//!
//!   cargo run --bin bt_auto_then_nuclear --features rfcomm
//!
//! PASS wenn soft ODER nuclear Linked liefert.

use reddot_desktop_lib::connection::run_nuclear_link;
use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_reddot_candidate},
    spp_com, target::RfcommTarget, RfcommSocket, WinsockRuntime,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn load_known() -> Option<RfcommTarget> {
    let path = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.reddot.arena")
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
        .expect("Kein Known/Candidate — JSON oder Bond nötig")
}

fn print_bond(addr: u64, label: &str) {
    match bond_state(addr) {
        Ok(Some(b)) => eprintln!(
            "  bond {label}: auth={} rem={} connected={}",
            b.authenticated, b.remembered, b.connected
        ),
        Ok(None) => eprintln!("  bond {label}: (none)"),
        Err(e) => eprintln!("  bond {label}: err {e}"),
    }
}

fn main() {
    eprintln!("=== bt_auto_then_nuclear (soft once → settle → nuclear) ===");
    eprintln!("Voraussetzung: Ziel AN und nah (auch für Pair nach Forget).");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let target = resolve();
    eprintln!(
        "Target {} @ {:012X}",
        target.display_name, target.bt_addr
    );
    print_bond(target.bt_addr, "start");

    let bonded = matches!(bond_state(target.bt_addr), Ok(Some(b)) if b.authenticated);
    if !bonded {
        eprintln!("kein Bond — überspringe soft, nur Nuclear…");
    } else {
        eprint!("[1] soft connect 12s… ");
        let t0 = Instant::now();
        match RfcommSocket::connect(&target, Duration::from_secs(12)) {
            Ok(sock) => {
                eprintln!("OK ({:.0?}) — Nuclear nicht nötig", t0.elapsed());
                std::thread::sleep(Duration::from_secs(2));
                drop(sock);
                eprintln!("PASS via Autoconnect once");
                return;
            }
            Err(e) => {
                eprintln!("FAIL ({:.0?}) — {e}", t0.elapsed());
                spp_com::restore_all();
                print_bond(target.bt_addr, "after soft-fail");
                eprintln!("  settle 8s (Stack/ADDRINUSE nach Abort)…");
                std::thread::sleep(Duration::from_secs(8));
            }
        }
    }

    eprintln!("[2] nuclear fallback…");
    print_bond(target.bt_addr, "before nuclear");
    let t1 = Instant::now();
    match run_nuclear_link(target.bt_addr, &target.display_name) {
        Ok((_t, sock)) => {
            eprintln!("OK nuclear ({:.0?})", t1.elapsed());
            print_bond(target.bt_addr, "linked");
            std::thread::sleep(Duration::from_secs(1));
            drop(sock);
            eprintln!("PASS via Nuclear after soft-fail");
        }
        Err(e) => {
            eprintln!("FAIL nuclear ({:.0?}): {e}", t1.elapsed());
            print_bond(target.bt_addr, "after nuclear-fail");
            eprintln!("HINT: Ziel an? Pairbar? BT-Adapter an? Danach bt_reset_connect allein testen.");
            std::process::exit(2);
        }
    }
}
