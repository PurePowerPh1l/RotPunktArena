//! Corrupt known-target JSON must not crash; manager stays usable.
//!
//!   cargo run --bin bt_json_corrupt --features rfcomm
//!
//! Writes invalid JSON, starts ConnectionManager, expects no panic and
//! NeedsTarget or Idle (not Linked). Restores backup if present.

use reddot_desktop_lib::connection::{ConnectionCommand, ConnectionManager, ConnectionStatus};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.reddot.arena")
}

fn main() {
    eprintln!("=== bt_json_corrupt ===");
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("rfcomm_known_target.json");
    let bak = dir.join("rfcomm_known_target.json.bak_lab");

    let previous = fs::read(&path).ok();
    if let Some(ref bytes) = previous {
        let _ = fs::write(&bak, bytes);
        eprintln!("backed up existing known → .bak_lab");
    }

    fs::write(&path, b"{ not valid json [[[").expect("write corrupt json");
    eprintln!("wrote corrupt JSON");

    let mgr = ConnectionManager::start(dir.clone(), None);
    let h = mgr.handle();
    std::thread::sleep(Duration::from_secs(2));
    let st = h.status();
    eprintln!("status={} reason={}", st.as_str(), h.last_reason());

    let linked = st == ConnectionStatus::Linked;
    let _ = h.send(ConnectionCommand::Shutdown);
    std::thread::sleep(Duration::from_millis(200));

    // Restore
    if let Some(bytes) = previous {
        let _ = fs::write(&path, bytes);
        eprintln!("restored previous known JSON");
    } else {
        let _ = fs::remove_file(&path);
        eprintln!("removed corrupt JSON (no prior known)");
    }

    if linked {
        eprintln!("FAIL Linked trotz korrupter JSON");
        std::process::exit(2);
    }
    eprintln!("PASS kein Crash, kein Linked");
}
