//! Geplanter Autoconnect: **ein** RFCOMM-Connect bei bestehendem Bond (kein Forget/Pair).
//!
//!   cargo run --bin bt_auto_once --features rfcomm
//!
//! PASS: Bonded + connect OK → Linked.
//! PASS SKIP (exit 0): Known da, aber kein authenticated Bond — Soft-Autoconnect N/A.
//! Exit 4: kein Known/Candidate.
//! Exit 2: Bonded aber Connect fail (Nuclear / N9).

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

fn resolve() -> Option<RfcommTarget> {
    load_known()
        .or_else(|| find_reddot_candidate().ok().flatten())
        .map(|mut t| {
            t.rfcomm_channel = Some(1);
            t
        })
}

fn main() {
    eprintln!("=== bt_auto_once (1× RFCOMM, kein Nuclear) ===");
    eprintln!("Voraussetzung: Ziel AN und nah, Bond authenticated.");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let Some(target) = resolve() else {
        eprintln!("FAIL kein Known/Candidate — Exit 4");
        std::process::exit(4);
    };
    eprintln!(
        "Target {} @ {:012X}",
        target.display_name, target.bt_addr
    );

    match bond_state(target.bt_addr) {
        Ok(Some(b)) if b.authenticated => {
            eprintln!(
                "bond: auth={} remembered={} connected={}",
                b.authenticated, b.remembered, b.connected
            );
        }
        Ok(Some(b)) => {
            eprintln!(
                "PASS SKIP: Bond nicht authenticated (auth={} rem={}) — Autoconnect N/A, Nuclear zuständig",
                b.authenticated, b.remembered
            );
            std::process::exit(0);
        }
        Ok(None) => {
            eprintln!(
                "PASS SKIP: Bond unknown (None) — Known-JSON ohne OS-Bond; Soft-Autoconnect N/A"
            );
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("PASS SKIP: bond_state err {e} — Soft-Autoconnect N/A");
            std::process::exit(0);
        }
    }

    // Soft path: try WITHOUT killing COM first (näher an „Kabel“); bei Fail einmal mit release.
    let page = Duration::from_secs(12);
    eprint!("[a] soft connect ohne spp_com release… ");
    let t0 = Instant::now();
    match RfcommSocket::connect(&target, page) {
        Ok(sock) => {
            eprintln!("OK ({:.0?})", t0.elapsed());
            std::thread::sleep(Duration::from_secs(2));
            drop(sock);
            eprintln!("PASS Autoconnect once → Linked");
            return;
        }
        Err(e) => {
            eprintln!("FAIL ({:.0?}) — {e}", t0.elapsed());
        }
    }

    match spp_com::release_channel_for(target.bt_addr) {
        spp_com::SppComAction::None => eprintln!("[b] spp_com: none"),
        spp_com::SppComAction::Disabled { com, .. } => eprintln!("[b] spp_com: disabled {com}"),
        spp_com::SppComAction::Busy { com, detail } => {
            eprintln!("[b] spp_com: busy {com} ({detail})")
        }
    }
    std::thread::sleep(Duration::from_secs(2));
    eprint!("[b] soft connect nach spp release… ");
    let t1 = Instant::now();
    match RfcommSocket::connect(&target, page) {
        Ok(sock) => {
            eprintln!("OK ({:.0?})", t1.elapsed());
            std::thread::sleep(Duration::from_secs(2));
            drop(sock);
            spp_com::restore_all();
            eprintln!("PASS Autoconnect once (nach COM-release) → Linked");
        }
        Err(e) => {
            eprintln!("FAIL ({:.0?}) — {e}", t1.elapsed());
            spp_com::restore_all();
            eprintln!("HINT: Soft reicht nicht — Nuclear (bt_auto_then_nuclear / Badge Verbinden)");
            std::process::exit(2);
        }
    }
}
