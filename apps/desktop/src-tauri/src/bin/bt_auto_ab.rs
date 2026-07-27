//! A/B Autoconnect Lab — Soft-Toast Hypothese (kein PIN-Hook).
//!
//!   A = RFCOMM ohne spp_com release  ← **Produkt-Start** (silent one-shot)
//!   B = release_channel_for + 2s + RFCOMM  ← nur Diagnose (Toast)
//!
//! Produkt nutzt ausschließlich A. B bleibt Lab, weil Windows-Soft-Toast.
//!
//!   cargo run --bin bt_auto_ab --features rfcomm
//!   cargo run --bin bt_auto_ab --features rfcomm -- a
//!   cargo run --bin bt_auto_ab --features rfcomm -- b
//!   cargo run --bin bt_auto_ab --features rfcomm -- both
//!
//! Env: REDDOT_AB=a|b|both (CLI-Arg hat Vorrang), REDDOT_PAGE_SECS=12
//!
//! Manuell: Zwischen den Phasen Enter — Soft-Toast ja/nein notieren.
//! Exit 0 = gewählte Phasen Linked (oder SKIP ohne Bond)
//! Exit 2 = Connect fail | Exit 4 = kein Target

use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_reddot_candidate},
    spp_com, target::RfcommTarget, RfcommSocket, WinsockRuntime,
};
use std::io::{self, Write};
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

fn resolve() -> Option<RfcommTarget> {
    load_known()
        .or_else(|| find_reddot_candidate().ok().flatten())
        .map(|mut t| {
            t.rfcomm_channel = Some(1);
            t
        })
}

fn page_secs() -> u64 {
    std::env::var("REDDOT_PAGE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(12)
}

fn mode_from_args() -> String {
    let arg = std::env::args().nth(1).unwrap_or_default();
    if !arg.is_empty() {
        return arg.to_ascii_lowercase();
    }
    std::env::var("REDDOT_AB")
        .unwrap_or_else(|_| "both".into())
        .to_ascii_lowercase()
}

fn wait_enter(prompt: &str) {
    eprint!("{prompt}");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
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

fn connect_once(target: &RfcommTarget, page: Duration, tag: &str) -> Result<(), String> {
    eprint!("  [{tag}] RfcommSocket::connect ({page:?})… ");
    let t0 = Instant::now();
    match RfcommSocket::connect(target, page) {
        Ok(sock) => {
            eprintln!("OK ({:.0?}) ch={}", t0.elapsed(), sock.channel.unwrap_or(1));
            eprintln!("  holding 2s then drop…");
            std::thread::sleep(Duration::from_secs(2));
            drop(sock);
            spp_com::restore_all();
            Ok(())
        }
        Err(e) => {
            eprintln!("FAIL ({:.0?}) — {e}", t0.elapsed());
            Err(e.to_string())
        }
    }
}

fn run_a(target: &RfcommTarget, page: Duration) -> Result<(), String> {
    eprintln!("\n======== PHASE A ========");
    eprintln!("  kein spp_com release, kein PIN-Hook");
    eprintln!("  >>> Soft-Toast? Jetzt beobachten (während connect) <<<");
    wait_enter("  Enter = A starten… ");
    print_bond(target.bt_addr, "a-before");
    let r = connect_once(target, page, "A");
    print_bond(target.bt_addr, "a-after");
    wait_enter("  Soft-Toast bei A? (Enter weiter)… ");
    r
}

fn run_b(target: &RfcommTarget, page: Duration) -> Result<(), String> {
    eprintln!("\n======== PHASE B ========");
    eprintln!("  release_channel_for → 2s settle → connect");
    eprintln!("  >>> Soft-Toast? Jetzt beobachten (release + connect) <<<");
    wait_enter("  Enter = B starten… ");
    print_bond(target.bt_addr, "b-before");
    match spp_com::release_channel_for(target.bt_addr) {
        spp_com::SppComAction::None => eprintln!("  spp_com: none"),
        spp_com::SppComAction::Disabled { com, .. } => {
            eprintln!("  spp_com: disabled {com}")
        }
        spp_com::SppComAction::Busy { com, detail } => {
            eprintln!("  spp_com: busy {com} ({detail})")
        }
    }
    eprintln!("  settle 2s…");
    std::thread::sleep(Duration::from_secs(2));
    let r = connect_once(target, page, "B");
    print_bond(target.bt_addr, "b-after");
    spp_com::restore_all();
    wait_enter("  Soft-Toast bei B? (Enter weiter)… ");
    r
}

fn main() {
    eprintln!("=== bt_auto_ab (A vs B, kein Hook) ===");
    eprintln!("Hypothese: B (Release+Connect) zieht Soft-Toast, A eher nicht.");
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
        Ok(Some(b)) => {
            eprintln!(
                "PASS SKIP: nicht authenticated (auth={} rem={})",
                b.authenticated, b.remembered
            );
            std::process::exit(0);
        }
        Ok(None) => {
            eprintln!("PASS SKIP: kein OS-Bond");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("PASS SKIP: bond_state err {e}");
            std::process::exit(0);
        }
    }

    let page = Duration::from_secs(page_secs());
    let mode = mode_from_args();
    eprintln!("mode={mode} page={page:?}");

    let mut a_ok = None;
    let mut b_ok = None;

    match mode.as_str() {
        "a" => {
            a_ok = Some(run_a(&target, page).is_ok());
        }
        "b" => {
            b_ok = Some(run_b(&target, page).is_ok());
        }
        "both" | "" => {
            a_ok = Some(run_a(&target, page).is_ok());
            eprintln!("\n  settle 5s zwischen A und B…");
            std::thread::sleep(Duration::from_secs(5));
            b_ok = Some(run_b(&target, page).is_ok());
        }
        other => {
            eprintln!("FAIL unknown mode {other:?} — use a | b | both");
            std::process::exit(1);
        }
    }

    eprintln!("\n======== SUMMARY ========");
    if let Some(ok) = a_ok {
        eprintln!("  A (no release): {}", if ok { "PASS Linked" } else { "FAIL" });
    }
    if let Some(ok) = b_ok {
        eprintln!(
            "  B (release+connect): {}",
            if ok { "PASS Linked" } else { "FAIL" }
        );
    }
    eprintln!("  Soft-Toast: manuell notieren (A? B? beide? keiner?)");

    let fail = a_ok == Some(false) || b_ok == Some(false);
    if fail {
        std::process::exit(2);
    }
    eprintln!("PASS bt_auto_ab");
}
