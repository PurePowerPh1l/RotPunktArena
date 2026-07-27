//! A/B Soft-Wake settle: real 12s pages, only the pause between fails differs.
//!
//!   cargo run --bin bt_softwake_ab --features softwake-labs
//!   (needs `rfcomm` from default features; Lab ≠ Produkt)
//!
//! Arm A: connect(12s) → fail → pause 3s  → …
//! Arm B: connect(12s) → fail → pause 12s → …
//!
//! Stops an arm early on first Linked. Counts ADDRINUSE / HOSTDOWN / TIMEOUT.
//! Env: REDDOT_AB_ATTEMPTS (default 8), REDDOT_AB_GAP_SECS between arms (default 20)

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

fn classify(err: &str) -> &'static str {
    if err.contains("10048") || err.contains("ADDRINUSE") {
        "ADDRINUSE"
    } else if err.contains("10064") || err.contains("HOSTDOWN") {
        "HOSTDOWN"
    } else if err.to_lowercase().contains("timeout") {
        "TIMEOUT"
    } else {
        "OTHER"
    }
}

#[derive(Default)]
struct Stats {
    attempts: u32,
    ok: bool,
    ok_ms: u128,
    addrinuse: u32,
    hostdown: u32,
    timeout: u32,
    other: u32,
    wall: Duration,
}

fn run_arm(label: &str, target: &RfcommTarget, pause: Duration, max_attempts: u32) -> Stats {
    eprintln!("\n=== ARM {label}: page=12s pause={pause:?} max={max_attempts} ===");
    let page = Duration::from_secs(12);
    let mut st = Stats::default();
    let wall0 = Instant::now();

    for i in 1..=max_attempts {
        st.attempts = i;
        eprint!("  [{i}/{max_attempts}] connect… ");
        let t0 = Instant::now();
        match RfcommSocket::connect(target, page) {
            Ok(s) => {
                let ms = t0.elapsed().as_millis();
                eprintln!("OK ({ms} ms)");
                st.ok = true;
                st.ok_ms = ms;
                drop(s);
                break;
            }
            Err(e) => {
                let c = classify(&e.to_string());
                eprintln!("{c} ({:.0?})", t0.elapsed());
                match c {
                    "ADDRINUSE" => st.addrinuse += 1,
                    "HOSTDOWN" => st.hostdown += 1,
                    "TIMEOUT" => st.timeout += 1,
                    _ => st.other += 1,
                }
                if i < max_attempts {
                    eprintln!("         pause {pause:?}…");
                    std::thread::sleep(pause);
                }
            }
        }
    }
    st.wall = wall0.elapsed();
    st
}

fn print_stats(label: &str, st: &Stats) {
    eprintln!(
        "--- {label}: linked={} attempts={} wall={:.0?}  ADDRINUSE={} HOSTDOWN={} TIMEOUT={} OTHER={}  ok_ms={}",
        st.ok,
        st.attempts,
        st.wall,
        st.addrinuse,
        st.hostdown,
        st.timeout,
        st.other,
        if st.ok {
            st.ok_ms.to_string()
        } else {
            "-".into()
        }
    );
}

fn main() {
    eprintln!("=== bt_softwake_ab (real pages, pause A/B) ===");
    eprintln!("Hypothesis: pause 12s vs 3s → fewer ADDRINUSE / faster Linked.");
    let _ = WinsockRuntime::init();

    let mut target = load_known().unwrap_or_else(|| {
        find_reddot_candidate()
            .ok()
            .flatten()
            .expect("Kein Known-Target / Candidate")
    });
    target.rfcomm_channel = Some(1);

    match bond_state(target.bt_addr) {
        Ok(Some(b)) if b.authenticated => {
            eprintln!(
                "Bond OK remembered={} connected={}",
                b.remembered, b.connected
            );
        }
        Ok(Some(_)) => {
            eprintln!("FAIL Bond nicht authenticated");
            std::process::exit(4);
        }
        Ok(None) => {
            eprintln!("FAIL Gerät unbekannt");
            std::process::exit(4);
        }
        Err(e) => {
            eprintln!("FAIL bond_state: {e}");
            std::process::exit(4);
        }
    }
    eprintln!(
        "Target {} @ {:012X}",
        target.display_name, target.bt_addr
    );

    let _ = spp_com::release_channel_for(target.bt_addr);

    let max_attempts: u32 = std::env::var("REDDOT_AB_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let gap = Duration::from_secs(
        std::env::var("REDDOT_AB_GAP_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20),
    );

    eprintln!("Initial quiet 10s…");
    std::thread::sleep(Duration::from_secs(10));

    let a = run_arm("A_pause3s", &target, Duration::from_secs(3), max_attempts);
    eprintln!("\nCool-down {gap:?} between arms…");
    std::thread::sleep(gap);
    let b = run_arm("B_pause12s", &target, Duration::from_secs(12), max_attempts);

    eprintln!("\n======== COMPARISON ========");
    print_stats("A pause=3s ", &a);
    print_stats("B pause=12s", &b);

    if !a.ok && !b.ok {
        eprintln!("INCONCLUSIVE: kein Linked in beiden Armen.");
        std::process::exit(2);
    }

    let fewer_busy = b.addrinuse < a.addrinuse;
    let better_link = b.ok && (!a.ok || b.attempts < a.attempts || b.wall < a.wall);
    if fewer_busy || better_link {
        eprintln!("RESULT: längere Pause sieht besser aus.");
        std::process::exit(0);
    }
    if a.addrinuse == b.addrinuse && a.ok == b.ok && a.attempts == b.attempts {
        eprintln!("RESULT: kein klarer Unterschied.");
        std::process::exit(1);
    }
    eprintln!("RESULT: längere Pause hilft hier nicht klar.");
    std::process::exit(1);
}
