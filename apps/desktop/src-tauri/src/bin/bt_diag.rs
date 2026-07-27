//! Bluetooth discovery + connect diagnostics (no UI).
//!   cargo run --bin bt_diag --features rfcomm

use reddot_desktop_lib::rfcomm::{
    discovery::{enumerate_paired_detailed, find_reddot_candidate},
    sdp::resolve_spp_channel,
    RfcommSocket, WinsockRuntime,
};
use std::time::Duration;

fn main() {
    eprintln!("=== RedDot BT discovery diag ===");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
    }

    match enumerate_paired_detailed() {
        Ok(report) => {
            eprintln!(
                "WSALookup: {} | Find: {} | PnP: {} | Merged: {}",
                report.wsalookup.len(),
                report.bt_find.len(),
                report.pnp.len(),
                report.merged.len()
            );
            for d in &report.merged {
                let mark = if d.display_name.to_uppercase().contains("RDT") {
                    " <<<"
                } else {
                    ""
                };
                eprintln!("  {:012X}  {:?}{mark}", d.bt_addr, d.display_name);
            }
        }
        Err(e) => eprintln!("enumerate error: {e}"),
    }

    let Some(t) = find_reddot_candidate().ok().flatten() else {
        eprintln!("No RedDot candidate — abort");
        return;
    };
    eprintln!("Candidate: {} @ {:012X}", t.display_name, t.bt_addr);

    match resolve_spp_channel(t.bt_addr) {
        Ok(Some(ch)) => eprintln!("SDP SPP channel: {ch}"),
        Ok(None) => eprintln!("SDP SPP channel: (none)"),
        Err(e) => eprintln!("SDP error: {e}"),
    }

    eprintln!("Trying RfcommSocket::connect (45s)…");
    match RfcommSocket::connect(&t, Duration::from_secs(45)) {
        Ok(_) => eprintln!("CONNECT OK"),
        Err(e) => eprintln!("CONNECT FAIL: {e}"),
    }
    eprintln!("=== done ===");
}
