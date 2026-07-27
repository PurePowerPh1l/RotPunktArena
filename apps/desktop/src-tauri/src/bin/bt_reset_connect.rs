//! Lab: Nuclear core only (Forget → Pair → RFCOMM) — twin of product `run_nuclear_link`.
//!
//!   cargo run --bin bt_reset_connect --features rfcomm

use reddot_desktop_lib::connection::run_nuclear_link;
use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_nearby_reddot, find_reddot_candidate},
    target::RfcommTarget, WinsockRuntime,
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

fn resolve_target() -> RfcommTarget {
    if let Some(mut t) = load_known() {
        t.rfcomm_channel = Some(1);
        return t;
    }
    if let Ok(Some(mut t)) = find_reddot_candidate() {
        t.rfcomm_channel = Some(1);
        return t;
    }
    if let Ok(Some(d)) = find_nearby_reddot() {
        return RfcommTarget {
            bt_addr: d.bt_addr,
            display_name: d.display_name,
            service_uuid: reddot_desktop_lib::rfcomm::SPP_SERVICE_UUID.to_string(),
            rfcomm_channel: Some(1),
            com_port: None,
        };
    }
    panic!("Kein RedDot — Ziel einschalten");
}

fn main() {
    eprintln!("=== bt_reset_connect (run_nuclear_link) ===");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }
    let target = resolve_target();
    eprintln!(
        "Target {} @ {:012X}",
        target.display_name, target.bt_addr
    );
    match bond_state(target.bt_addr) {
        Ok(Some(b)) => eprintln!(
            "bond before: auth={} connected={}",
            b.authenticated, b.connected
        ),
        _ => eprintln!("bond before: unknown"),
    }

    let t0 = Instant::now();
    match run_nuclear_link(target.bt_addr, &target.display_name) {
        Ok((t, sock)) => {
            eprintln!(
                "PASS Linked {} in {:.0?}",
                t.display_name,
                t0.elapsed()
            );
            std::thread::sleep(Duration::from_secs(1));
            drop(sock);
        }
        Err(e) => {
            eprintln!("FAIL {e} wall={:.0?}", t0.elapsed());
            std::process::exit(2);
        }
    }
}
