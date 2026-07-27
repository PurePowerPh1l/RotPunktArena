//! One-shot classic BT pair for RedDot (PIN 0000) — recovery / lab use.
//!   cargo run --bin bt_pair --features rfcomm

use reddot_desktop_lib::rfcomm::{
    auth_hook,
    discovery::{bond_state, find_nearby_reddot, find_reddot_candidate},
    WinsockRuntime,
};

const PIN: &str = "0000";
const ADDR: u64 = 0x0018_DA07_0564;

fn main() {
    eprintln!("=== bt_pair ===");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let (addr, name) = if let Some(t) = find_reddot_candidate().ok().flatten() {
        (t.bt_addr, t.display_name)
    } else if let Some(d) = find_nearby_reddot().ok().flatten() {
        (d.bt_addr, d.display_name)
    } else {
        (ADDR, "KT RDT ZIE 1".into())
    };
    eprintln!("Target: {name} @ {addr:012X}");

    if let Ok(Some(b)) = bond_state(addr) {
        eprintln!(
            "bond before: auth={} remembered={} connected={}",
            b.authenticated, b.remembered, b.connected
        );
        if b.authenticated {
            eprintln!("Already authenticated — nothing to do");
            return;
        }
    } else {
        eprintln!("bond before: (unknown)");
    }

    eprintln!("pair_with_pin_exclusive PIN={PIN}…");
    match auth_hook::pair_with_pin_exclusive(addr, &name, PIN) {
        Ok(()) => eprintln!("pair_with_pin_exclusive: Ok"),
        Err(e) => {
            eprintln!("pair_with_pin_exclusive FAIL: {e}");
            std::process::exit(2);
        }
    }

    std::thread::sleep(std::time::Duration::from_secs(2));
    match bond_state(addr) {
        Ok(Some(b)) => {
            eprintln!(
                "bond after: auth={} remembered={} connected={}",
                b.authenticated, b.remembered, b.connected
            );
            if !b.authenticated {
                eprintln!("FAIL: not authenticated after pair");
                std::process::exit(3);
            }
        }
        Ok(None) => {
            eprintln!("FAIL: device not found after pair");
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("FAIL bond_state: {e}");
            std::process::exit(3);
        }
    }
    eprintln!("=== done ===");
}
