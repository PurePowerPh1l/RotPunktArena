//! Minimal „gekoppelt → verbinden“ Lab (Einfach ≈ Genial).
//!
//! Voraussetzung: Gerät ist schon gepaart (authenticated). Kein Pairing, kein
//! Soft-Wake-Orchester — nur:
//!   COM einmal freigeben → connect → Fail → Pause → nochmal.
//! Bei ADDRINUSE: einmal extra COM freigeben + etwas länger warten.
//!
//!   cargo run --bin bt_bonded_simple --features rfcomm
//!
//! Env (optional):
//!   REDDOT_ATTEMPTS   default 20
//!   REDDOT_PAGE_SECS  connect-Timeout pro Versuch (default 12)
//!   REDDOT_PAUSE_SECS Pause nach Fail (default 3)

use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_reddot_candidate},
    spp_com::{self, SppComAction},
    target::RfcommTarget, RfcommSocket, WinsockRuntime,
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

fn release_com(addr: u64, why: &str) {
    match spp_com::release_channel_for(addr) {
        SppComAction::None => eprintln!("  spp_com ({why}): none"),
        SppComAction::Disabled { com, .. } => eprintln!("  spp_com ({why}): disabled {com}"),
        SppComAction::Busy { com, detail } => {
            eprintln!("  spp_com ({why}): busy {com} ({detail})")
        }
    }
}

fn print_bond(label: &str, addr: u64) {
    match bond_state(addr) {
        Ok(Some(b)) => eprintln!(
            "  bond {label}: auth={} remembered={} connected={}",
            b.authenticated, b.remembered, b.connected
        ),
        Ok(None) => eprintln!("  bond {label}: (device unknown)"),
        Err(e) => eprintln!("  bond {label}: err {e}"),
    }
}

fn main() {
    eprintln!("=== bt_bonded_simple (gekoppelt → connect loop) ===");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let mut target = load_known().unwrap_or_else(|| {
        find_reddot_candidate()
            .ok()
            .flatten()
            .expect("Kein Known-Target / Candidate — erst Setup/Pair")
    });
    target.rfcomm_channel = Some(1);

    eprintln!(
        "Target {} @ {:012X} ch={}",
        target.display_name,
        target.bt_addr,
        target.rfcomm_channel.unwrap_or(0)
    );
    print_bond("before", target.bt_addr);

    match bond_state(target.bt_addr) {
        Ok(Some(b)) if b.authenticated => {}
        Ok(Some(_)) => {
            eprintln!("FAIL: nicht authenticated — erst bt_pair / Setup");
            std::process::exit(4);
        }
        Ok(None) => {
            eprintln!("FAIL: Gerät unbekannt in Windows — erst koppeln");
            std::process::exit(4);
        }
        Err(e) => {
            eprintln!("FAIL bond_state: {e}");
            std::process::exit(4);
        }
    }

    let max = env_u32("REDDOT_ATTEMPTS", 20);
    let page = Duration::from_secs(u64::from(env_u32("REDDOT_PAGE_SECS", 12)));
    let pause = Duration::from_secs(u64::from(env_u32("REDDOT_PAUSE_SECS", 3)));
    eprintln!("page={page:?} pause={pause:?} max_attempts={max}");

    release_com(target.bt_addr, "start");

    let wall0 = Instant::now();
    let mut addrinuse = 0u32;
    let mut hostdown = 0u32;
    let mut timeout = 0u32;
    let mut other = 0u32;
    let mut freed_on_busy = false;

    for i in 1..=max {
        eprint!("  [{i}/{max}] connect… ");
        let t0 = Instant::now();
        match RfcommSocket::connect(&target, page) {
            Ok(sock) => {
                let ms = t0.elapsed().as_millis();
                eprintln!("OK ({ms} ms) wall={:.0?}", wall0.elapsed());
                print_bond("linked", target.bt_addr);
                eprintln!("  holding socket 2s then clean drop…");
                std::thread::sleep(Duration::from_secs(2));
                drop(sock);
                spp_com::restore_all();
                std::thread::sleep(Duration::from_millis(500));
                print_bond("after drop", target.bt_addr);
                eprintln!(
                    "PASS Linked after {i} attempt(s)  \
                     ADDRINUSE={addrinuse} HOSTDOWN={hostdown} TIMEOUT={timeout} OTHER={other}"
                );
                return;
            }
            Err(e) => {
                let msg = e.to_string();
                let c = classify(&msg);
                eprintln!("{c} ({:.0?}) — {msg}", t0.elapsed());
                match c {
                    "ADDRINUSE" => {
                        addrinuse += 1;
                        if !freed_on_busy {
                            freed_on_busy = true;
                            release_com(target.bt_addr, "ADDRINUSE");
                            eprintln!("         extra settle 5s…");
                            std::thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                    }
                    "HOSTDOWN" => hostdown += 1,
                    "TIMEOUT" => timeout += 1,
                    "ACCES" => {
                        eprintln!("FAIL WSAEACCES — kein Auto-Pair in diesem Test");
                        spp_com::restore_all();
                        std::process::exit(5);
                    }
                    _ => other += 1,
                }
                if i < max {
                    eprintln!("         pause {pause:?}…");
                    std::thread::sleep(pause);
                }
            }
        }
    }

    spp_com::restore_all();
    print_bond("after fail", target.bt_addr);
    eprintln!(
        "FAIL no Linked in {max} attempts (wall={:.0?})  \
         ADDRINUSE={addrinuse} HOSTDOWN={hostdown} TIMEOUT={timeout} OTHER={other}",
        wall0.elapsed()
    );
    std::process::exit(2);
}
