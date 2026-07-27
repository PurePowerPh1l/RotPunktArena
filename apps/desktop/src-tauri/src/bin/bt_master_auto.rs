//! Master-style Autoconnect Lab: PIN-Hook + Soft-Wake-Schleife (kein Nuclear).
//!
//! Wie `master` ConnectionManager bei Known+Bonded:
//!   install_reddot_pin_hook → RFCOMM connect(12s) → Fail → Pause 3s → … Cap 10
//!
//!   cargo run --bin bt_master_auto --features softwake-labs
//!   (needs `rfcomm` from default features; Lab ≠ Produkt / comparison-only)
//!
//! Env: REDDOT_SOFT_CAP (10), REDDOT_PAGE_SECS (12), REDDOT_PAUSE_SECS (3)
//!
//! Exit 0 PASS Linked | 2 kein Linked in Cap | 4 kein Bond/Known

use reddot_desktop_lib::rfcomm::{
    auth_hook,
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

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn resolve() -> Option<RfcommTarget> {
    load_known()
        .or_else(|| find_reddot_candidate().ok().flatten())
        .map(|mut t| {
            t.rfcomm_channel = Some(1);
            t
        })
}

fn classify(err: &str) -> &'static str {
    if err.contains("10048") || err.contains("ADDRINUSE") {
        "ADDRINUSE"
    } else if err.contains("10064") || err.contains("HOSTDOWN") {
        "HOSTDOWN"
    } else if err.to_lowercase().contains("timeout") {
        "TIMEOUT"
    } else if err.contains("10013") || err.contains("ACCES") {
        "ACCES"
    } else {
        "OTHER"
    }
}

fn main() {
    eprintln!("=== bt_master_auto (Hook + Soft-Wake, wie Master) ===");
    eprintln!("Kein Forget/Pair — nur Paging mit Auto-PIN 0000.");
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
                "bond: auth={} rem={} connected={}",
                b.authenticated, b.remembered, b.connected
            );
        }
        Ok(Some(_)) => {
            eprintln!("FAIL Bond nicht authenticated — Master würde NeedsPairing (kein Auto)");
            std::process::exit(4);
        }
        _ => {
            eprintln!("FAIL kein OS-Bond — erst Nuclear/Setup, dann dieser Lab");
            std::process::exit(4);
        }
    }

    // Permanent hook like ConnectionManager::start on master.
    if let Err(e) = auth_hook::install_reddot_pin_hook() {
        eprintln!("FAIL install PIN hook: {e}");
        std::process::exit(1);
    }
    auth_hook::allow_auto_pin_for(target.bt_addr);
    eprintln!("PIN hook installed + allow {:012X}", target.bt_addr);

    match spp_com::release_channel_for(target.bt_addr) {
        spp_com::SppComAction::None => eprintln!("spp_com: none"),
        spp_com::SppComAction::Disabled { com, .. } => eprintln!("spp_com: disabled {com}"),
        spp_com::SppComAction::Busy { com, detail } => {
            eprintln!("spp_com: busy {com} ({detail})")
        }
    }

    let cap = env_u32("REDDOT_SOFT_CAP", 10);
    let page = Duration::from_secs(u64::from(env_u32("REDDOT_PAGE_SECS", 12)));
    let pause = Duration::from_secs(u64::from(env_u32("REDDOT_PAUSE_SECS", 3)));
    eprintln!("soft-wake: page={page:?} pause={pause:?} cap={cap}");

    let wall0 = Instant::now();
    for i in 1..=cap {
        eprint!("  [{i}/{cap}] connect… ");
        let t0 = Instant::now();
        match RfcommSocket::connect(&target, page) {
            Ok(sock) => {
                eprintln!("OK ({:.0?}) wall={:.0?}", t0.elapsed(), wall0.elapsed());
                eprintln!("  hold 3s…");
                std::thread::sleep(Duration::from_secs(3));
                drop(sock);
                spp_com::restore_all();
                auth_hook::clear_auto_pin_allows();
                auth_hook::uninstall_reddot_pin_hook();
                eprintln!("PASS Master-style Soft-Wake Linked after {i} attempt(s)");
                return;
            }
            Err(e) => {
                let msg = e.to_string();
                let c = classify(&msg);
                eprintln!("{c} ({:.0?}) — {msg}", t0.elapsed());
                if c == "ACCES" {
                    eprintln!("FAIL WSAEACCES — Master stoppt hier (kein Spam)");
                    spp_com::restore_all();
                    auth_hook::clear_auto_pin_allows();
                    auth_hook::uninstall_reddot_pin_hook();
                    std::process::exit(5);
                }
                if i < cap {
                    eprintln!("         pause {pause:?}…");
                    std::thread::sleep(pause);
                }
            }
        }
    }

    spp_com::restore_all();
    auth_hook::clear_auto_pin_allows();
    auth_hook::uninstall_reddot_pin_hook();
    eprintln!(
        "FAIL kein Linked in {cap} Soft-Wake-Versuchen wall={:.0?}",
        wall0.elapsed()
    );
    std::process::exit(2);
}
