//! Bond-Gate Autoconnect Matrix (vor Produkteinbau).
//!
//! Soll-Staffelung:
//!   Bond OK  → Soft (Hook + Soft-Wake), kein Forget
//!   Bond weg → Soft N/A → Nuclear
//!
//!   cargo run --bin bt_bond_gate_matrix --features rfcomm
//!
//! Phasen:
//!   P1 Nuclear   — sauberer Bond+Link herstellen, Socket droppen
//!   P2 Soft      — Soft-Wake (Hook) bei Bond → Linked
//!   P3 Forget    — nur OS-Bond entfernen (Known-JSON bleibt)
//!   P4 Gate      — kein Bond → Soft-Pfad wird nicht gewählt (Nuclear zuständig)
//!   P5 Nuclear   — Recovery → Linked
//!
//! Hinweis: Soft+PIN-Hook kann ohne vorherigen Bond trotzdem linken
//! (Pairing während AF_BTH-Connect). Deshalb Gate = Bond-Check *vor*
//! Soft, nicht „Soft muss physisch scheitern“.
//!
//! Exit 0 = alle Phasen PASS | sonst erste FAIL-Phase.

use reddot_desktop_lib::connection::{forget_reddot_bonds, run_nuclear_link};
use reddot_desktop_lib::rfcomm::{
    auth_hook,
    discovery::{bond_state, find_reddot_candidate, remove_bond},
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
        .expect("Kein Known/Candidate — Ziel an, ggf. einmal Setup")
}

fn bond_auth(addr: u64) -> bool {
    matches!(bond_state(addr), Ok(Some(b)) if b.authenticated)
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

fn soft_wake_once_cap(
    target: &RfcommTarget,
    cap: u32,
    page: Duration,
    pause: Duration,
) -> Result<u32, String> {
    let _ = auth_hook::install_reddot_pin_hook().map_err(|e| e.to_string())?;
    auth_hook::allow_auto_pin_for(target.bt_addr);
    let _ = spp_com::release_channel_for(target.bt_addr);

    for i in 1..=cap {
        eprint!("    soft [{i}/{cap}]… ");
        let t0 = Instant::now();
        match RfcommSocket::connect(target, page) {
            Ok(sock) => {
                eprintln!("OK ({:.0?})", t0.elapsed());
                std::thread::sleep(Duration::from_secs(2));
                drop(sock);
                spp_com::restore_all();
                auth_hook::clear_auto_pin_allows();
                auth_hook::uninstall_reddot_pin_hook();
                return Ok(i);
            }
            Err(e) => {
                let msg = e.to_string();
                eprintln!("FAIL ({:.0?}) — {msg}", t0.elapsed());
                if msg.contains("10013") || msg.contains("ACCES") {
                    spp_com::restore_all();
                    auth_hook::clear_auto_pin_allows();
                    auth_hook::uninstall_reddot_pin_hook();
                    return Err(format!("WSAEACCES: {msg}"));
                }
                if i < cap {
                    std::thread::sleep(pause);
                }
            }
        }
    }
    spp_com::restore_all();
    auth_hook::clear_auto_pin_allows();
    auth_hook::uninstall_reddot_pin_hook();
    Err(format!("kein Linked in {cap} Soft-Versuchen"))
}

fn phase(name: &str) {
    eprintln!("\n======== {name} ========");
}

fn main() {
    eprintln!("=== bt_bond_gate_matrix ===");
    eprintln!("Soll: Bond OK → Soft-Autoconnect; Bond weg → Nuclear");
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

    let page = Duration::from_secs(12);
    let pause = Duration::from_secs(3);
    let soft_cap = 10u32;

    // --- P1: Nuclear establish ---
    phase("P1 Nuclear establish (sauberer Bond+Link)");
    {
        let t0 = Instant::now();
        match run_nuclear_link(target.bt_addr, &target.display_name) {
            Ok((_t, sock)) => {
                eprintln!("  PASS Linked ({:.0?})", t0.elapsed());
                print_bond(target.bt_addr, "p1");
                std::thread::sleep(Duration::from_secs(2));
                drop(sock);
                eprintln!("  socket dropped (App-Close simuliert)");
            }
            Err(e) => {
                eprintln!("FAIL P1 nuclear: {e}");
                std::process::exit(11);
            }
        }
    }
    eprintln!("  settle 5s vor Soft-Reopen…");
    std::thread::sleep(Duration::from_secs(5));

    // --- P2: Soft with bond ---
    phase("P2 Soft-Autoconnect (Bond muss OK sein)");
    print_bond(target.bt_addr, "p2-before");
    if !bond_auth(target.bt_addr) {
        eprintln!("FAIL P2: nach Nuclear kein authenticated Bond — Gate kaputt");
        std::process::exit(12);
    }
    match soft_wake_once_cap(&target, soft_cap, page, pause) {
        Ok(n) => eprintln!("  PASS Soft Linked nach {n} Versuch(en)"),
        Err(e) => {
            eprintln!("FAIL P2 soft (Bond war OK): {e}");
            eprintln!("  → Soft-Autoconnect bei gutem Bond trägt hier nicht");
            std::process::exit(12);
        }
    }
    print_bond(target.bt_addr, "p2-after");
    eprintln!("  settle 5s…");
    std::thread::sleep(Duration::from_secs(5));

    // --- P3: Remove bond only ---
    phase("P3 Forget OS-Bond (JSON/Known bleibt)");
    let _ = remove_bond(target.bt_addr);
    // Also clear name-hint bonds for cleanliness
    forget_reddot_bonds(target.bt_addr);
    print_bond(target.bt_addr, "p3");
    if bond_auth(target.bt_addr) {
        eprintln!("FAIL P3: Bond immer noch authenticated nach remove");
        std::process::exit(13);
    }
    eprintln!("  PASS Bond weg");
    std::thread::sleep(Duration::from_secs(2));

    // --- P4: Product gate — no Bond ⇒ Soft path not taken ---
    phase("P4 Gate: kein Bond → Soft überspringen (Nuclear wählen)");
    print_bond(target.bt_addr, "p4");
    if bond_auth(target.bt_addr) {
        eprintln!("FAIL P4: Bond wieder da — Gate-Voraussetzung falsch");
        std::process::exit(14);
    }
    // Produktentscheidung (noch nicht im Owner): choose_path(bond)
    let path = if bond_auth(target.bt_addr) {
        "soft"
    } else {
        "nuclear"
    };
    eprintln!("  choose_path(bond_auth=false) → {path}");
    if path != "nuclear" {
        eprintln!("FAIL P4: Soft gewählt trotz fehlendem Bond");
        std::process::exit(14);
    }
    eprintln!("  PASS Soft nicht gewählt — Nuclear zuständig");
    eprintln!("  (Hinweis: Soft+Hook *könnte* trotzdem linken via Pair-on-Connect;");
    eprintln!("   genau deshalb Gate vor Soft, nicht Soft-Versuch ohne Bond)");

    // --- P5: Nuclear recovery ---
    phase("P5 Nuclear Recovery");
    eprintln!("  settle 3s…");
    std::thread::sleep(Duration::from_secs(3));
    let t1 = Instant::now();
    match run_nuclear_link(target.bt_addr, &target.display_name) {
        Ok((_t, sock)) => {
            eprintln!("  PASS Nuclear Linked ({:.0?})", t1.elapsed());
            print_bond(target.bt_addr, "p5");
            std::thread::sleep(Duration::from_secs(1));
            drop(sock);
        }
        Err(e) => {
            eprintln!("FAIL P5 nuclear: {e}");
            std::process::exit(15);
        }
    }

    eprintln!("\n======== SUMMARY ========");
    eprintln!("P1 Nuclear establish     PASS");
    eprintln!("P2 Soft with Bond        PASS");
    eprintln!("P3 Forget Bond           PASS");
    eprintln!("P4 Gate skip Soft        PASS (nuclear chosen)");
    eprintln!("P5 Nuclear recovery      PASS");
    eprintln!("PASS bond-gate matrix — Soft nur bei Bond-Gate, sonst Nuclear");
}
