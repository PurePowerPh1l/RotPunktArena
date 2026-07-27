//! Connected-Gate Autoconnect Lab (Silent Start Hypothese).
//!
//! Soft-Toast nach langem Idle kommt oft schon von einem AF_BTH-Versuch gegen
//! ein schlafendes Gerät. Gate:
//!
//!   authenticated && connected  → genau 1× RFCOMM (Lab A, kein Hook/Release)
//!   sonst                       → SKIP, **kein** Connect (kein Toast erwartet)
//!
//!   cargo run --bin bt_auto_connected_gate --features rfcomm
//!   cargo run --bin bt_auto_connected_gate --features rfcomm -- force
//!     (force = trotz connected=false einmal A — Soft-Toast vergleichen)
//!
//! Manuell: Soft-Toast ja/nein notieren. Exit 0 = Linked oder SKIP | 2 = fail | 4 = kein Target

use reddot_desktop_lib::rfcomm::{
    discovery::{bond_state, find_reddot_candidate, BondState},
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

fn wait_enter(prompt: &str) {
    eprint!("{prompt}");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}

fn print_bond(label: &str, b: &BondState) {
    eprintln!(
        "  bond {label}: auth={} rem={} connected={}",
        b.authenticated, b.remembered, b.connected
    );
}

fn main() {
    eprintln!("=== bt_auto_connected_gate ===");
    eprintln!("Gate: auth+connected → 1× RFCOMM; sonst SKIP (kein Connect).");
    eprintln!("force = Connect trotz connected=false (Toast-Vergleich).");

    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let force = std::env::args()
        .nth(1)
        .map(|s| s.eq_ignore_ascii_case("force"))
        .unwrap_or(false);

    let Some(target) = resolve() else {
        eprintln!("FAIL kein Known/Candidate — Exit 4");
        std::process::exit(4);
    };
    eprintln!(
        "Target {} @ {:012X} force={force}",
        target.display_name, target.bt_addr
    );

    let bond = match bond_state(target.bt_addr) {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!("PASS SKIP: kein OS-Bond — kein Connect");
            wait_enter("  Soft-Toast ohne Connect? (Enter)… ");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("PASS SKIP: bond_state err {e} — kein Connect");
            wait_enter("  Soft-Toast ohne Connect? (Enter)… ");
            std::process::exit(0);
        }
    };
    print_bond("start", &bond);

    if !bond.authenticated {
        eprintln!("PASS SKIP: nicht authenticated — Nuclear / Setup zuständig");
        wait_enter("  Soft-Toast ohne Connect? (Enter)… ");
        std::process::exit(0);
    }

    if !bond.connected && !force {
        eprintln!("PASS SKIP: authenticated aber connected=false (Gerät idle/schlafend)");
        eprintln!("  → Produkt würde Idle + Badge Verbinden (kein AF_BTH, kein Toast erwartet)");
        wait_enter("  Soft-Toast bei SKIP? (Enter)… ");
        eprintln!("SUMMARY: gate_skip connected=false | Soft-Toast? manuell");
        std::process::exit(0);
    }

    if !bond.connected && force {
        eprintln!("FORCE: connected=false — trotzdem A (Toast-Hypothese Idle→AF_BTH)");
    } else {
        eprintln!("GATE OPEN: auth+connected — stiller One-Shot A");
    }

    eprintln!("  >>> Soft-Toast während connect beobachten <<<");
    wait_enter("  Enter = Connect starten… ");

    let page = Duration::from_secs(12);
    eprint!("  [A] RfcommSocket::connect ({page:?})… ");
    let t0 = Instant::now();
    match RfcommSocket::connect(&target, page) {
        Ok(sock) => {
            eprintln!("OK ({:.0?}) ch={}", t0.elapsed(), sock.channel.unwrap_or(1));
            print_bond("linked", &bond_state(target.bt_addr).ok().flatten().unwrap_or(bond));
            std::thread::sleep(Duration::from_secs(2));
            drop(sock);
            spp_com::restore_all();
            wait_enter("  Soft-Toast bei Connect? (Enter)… ");
            eprintln!("SUMMARY: PASS Linked | Soft-Toast? manuell");
            eprintln!("PASS bt_auto_connected_gate");
        }
        Err(e) => {
            eprintln!("FAIL ({:.0?}) — {e}", t0.elapsed());
            wait_enter("  Soft-Toast trotz Fail? (Enter)… ");
            eprintln!("SUMMARY: FAIL connect | Soft-Toast? manuell");
            std::process::exit(2);
        }
    }
}
