//! Nuclear against unreachable BD_ADDR — must fail cleanly (no hang).
//!
//!   cargo run --bin bt_nuclear_timeout --features rfcomm
//!
//! PASS (exit 0): `run_nuclear_link` returns Err within WALL_MAX.
//! FAIL (exit 2): Linked somehow / exit 3: wall exceeded.

use reddot_desktop_lib::connection::run_nuclear_link;
use reddot_desktop_lib::rfcomm::WinsockRuntime;
use std::time::{Duration, Instant};

/// Unlikely to be a live RedDot; still a valid 48-bit form.
const DEAD_ADDR: u64 = 0x0018_DA_FF_FF_FE;
const WALL_MAX: Duration = Duration::from_secs(120);

fn main() {
    eprintln!("=== bt_nuclear_timeout (expect FAIL from nuclear) ===");
    eprintln!("dead addr={DEAD_ADDR:012X} wall_max={WALL_MAX:?}");
    if let Err(e) = WinsockRuntime::init() {
        eprintln!("WSAStartup failed: {e}");
        std::process::exit(1);
    }

    let t0 = Instant::now();
    let result = run_nuclear_link(DEAD_ADDR, "RedDot TIMEOUT-LAB");
    let wall = t0.elapsed();
    eprintln!("wall={wall:?}");

    if wall > WALL_MAX {
        eprintln!("FAIL Hänger: wall > {WALL_MAX:?}");
        std::process::exit(3);
    }
    match result {
        Ok(_) => {
            eprintln!("FAIL unerwartet Linked zu Tot-Addr");
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("nuclear returned Err: {e}");
            if e.trim().is_empty() {
                eprintln!("FAIL Fehler ohne Reason");
                std::process::exit(4);
            }
            eprintln!("PASS erwarteter Fehler mit Reason");
        }
    }
}
