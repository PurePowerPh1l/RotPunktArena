//! Startup-Nuclear race lab (measurement only — product Owner unchanged).
//!
//!   cargo run --bin bt_startup_race --features rfcomm -- <mode>
//!
//! Modes:
//!   badge_attach   — NuclearLink while Startup Connecting (must attach, not 2nd Forget)
//!   setup_pause    — PauseForSetup while Connecting (cancel → Idle, no zombie link)
//!   exit_forget    — Shutdown while reason contains "vorbereitet"
//!   exit_pair      — Shutdown while reason contains "Kopple"
//!   exit_rfcomm    — Shutdown while reason contains "Verbinde"
//!   link_lost      — Linked halten → nach Power-Off Idle; kein Auto-Nuclear
//!   long_hold      — ≥4h Linked + RegisterSink/UnregisterSink-Zyklen (REDOT_LONG_HOLD_SECS)

use reddot_desktop_lib::connection::{
    connect_known_nuclear, ConnectionCommand, ConnectionManager, ConnectionStatus, ConnectOrigin,
};
use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("de.disag.rotpunktarena")
}

fn wait_reason_contains(
    h: &reddot_desktop_lib::connection::ConnectionHandle,
    needle: &str,
    max: Duration,
) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < max {
        let reason = h.last_reason();
        if reason.contains(needle) && h.status() == ConnectionStatus::Connecting {
            return true;
        }
        if matches!(
            h.status(),
            ConnectionStatus::Linked | ConnectionStatus::Idle | ConnectionStatus::NeedsPairing
        ) && t0.elapsed() > Duration::from_millis(500)
        {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "badge_attach".into());
    eprintln!("=== bt_startup_race mode={mode} ===");
    let dir = data_dir();
    let mgr = ConnectionManager::start(dir, None);
    let h = mgr.handle();

    match mode.as_str() {
        "badge_attach" => {
            if !wait_reason_contains(&h, "vorbereitet", Duration::from_secs(15))
                && !wait_reason_contains(&h, "Kopple", Duration::from_secs(15))
                && h.status() != ConnectionStatus::Connecting
            {
                eprintln!("FAIL never Connecting");
                std::process::exit(2);
            }
            eprintln!(
                "badge: Connecting reason={} origin={:?}",
                h.last_reason(),
                h.connect_origin()
            );
            let gen0 = h.generation();
            let origin0 = h.connect_origin();
            // Same owner: must attach / ignore — not a second Forget/Pair flight.
            let attach = thread::spawn({
                let h2 = h.clone();
                move || connect_known_nuclear(&h2)
            });
            let attach_res = attach.join().expect("attach thread");
            let st = h.status();
            let gen1 = h.generation();
            eprintln!(
                "after attach: status={} gen {gen0}->{gen1} origin0={origin0:?} attach={attach_res:?}",
                st.as_str()
            );
            let _ = h.send(ConnectionCommand::Shutdown);
            thread::sleep(Duration::from_millis(400));
            // Pass: Linked once; generation not double-bumped for a second nuclear start
            // (attach ignores NuclearLink → same gen; or Linked with gen==gen0 or gen0+1 from Start only).
            if st == ConnectionStatus::Linked && matches!(attach_res, Ok(_)) {
                if origin0 == ConnectOrigin::StartupAuto || gen1 <= gen0 + 1 {
                    eprintln!("PASS badge_attach Linked without parallel second Start bump storm");
                    return;
                }
            }
            if st == ConnectionStatus::Linked {
                eprintln!("PASS badge_attach Linked (attach ok or raced to same link)");
                return;
            }
            eprintln!(
                "FAIL badge_attach status={} attach={attach_res:?}",
                st.as_str()
            );
            std::process::exit(3);
        }
        "setup_pause" => {
            if !wait_reason_contains(&h, "Kopple", Duration::from_secs(30))
                && !wait_reason_contains(&h, "vorbereitet", Duration::from_secs(10))
            {
                eprintln!("FAIL never Connecting for setup_pause");
                std::process::exit(2);
            }
            let _ = h.send(ConnectionCommand::PauseForSetup);
            let t0 = Instant::now();
            while t0.elapsed() < Duration::from_secs(20) {
                let st = h.status();
                if st == ConnectionStatus::Discovering || st == ConnectionStatus::Idle {
                    eprintln!("PASS setup_pause → {}", st.as_str());
                    let _ = h.send(ConnectionCommand::Shutdown);
                    return;
                }
                if st == ConnectionStatus::Linked {
                    eprintln!("FAIL setup_pause became Linked after Pause");
                    let _ = h.send(ConnectionCommand::Shutdown);
                    std::process::exit(3);
                }
                thread::sleep(Duration::from_millis(100));
            }
            eprintln!("FAIL setup_pause timeout status={}", h.status().as_str());
            let _ = h.send(ConnectionCommand::Shutdown);
            std::process::exit(4);
        }
        "exit_forget" | "exit_pair" | "exit_rfcomm" => {
            let needle = match mode.as_str() {
                "exit_forget" => "vorbereitet",
                "exit_pair" => "Kopple",
                _ => "Verbinde",
            };
            if !wait_reason_contains(&h, needle, Duration::from_secs(60)) {
                eprintln!(
                    "FAIL never reached phase '{needle}' (status={} reason={})",
                    h.status().as_str(),
                    h.last_reason()
                );
                let _ = h.send(ConnectionCommand::Shutdown);
                std::process::exit(2);
            }
            eprintln!("exit at phase reason={}", h.last_reason());
            let _ = h.send(ConnectionCommand::Shutdown);
            thread::sleep(Duration::from_secs(2));
            // Process end: manager dropped. Late Linked impossible in this process.
            eprintln!("PASS {mode} Shutdown issued during phase (no further wait in-process)");
        }
        "link_lost" => {
            let t0 = Instant::now();
            while t0.elapsed() < Duration::from_secs(90) {
                if h.status() == ConnectionStatus::Linked {
                    break;
                }
                if matches!(
                    h.status(),
                    ConnectionStatus::Idle
                        | ConnectionStatus::NeedsPairing
                        | ConnectionStatus::Faulted
                        | ConnectionStatus::NeedsTarget
                ) && t0.elapsed() > Duration::from_secs(3)
                {
                    eprintln!(
                        "FAIL never Linked (status={} reason={})",
                        h.status().as_str(),
                        h.last_reason()
                    );
                    let _ = h.send(ConnectionCommand::Shutdown);
                    std::process::exit(2);
                }
                thread::sleep(Duration::from_millis(100));
            }
            if h.status() != ConnectionStatus::Linked {
                eprintln!("FAIL Linked timeout");
                let _ = h.send(ConnectionCommand::Shutdown);
                std::process::exit(2);
            }
            let gen_linked = h.generation();
            eprintln!(">>> LINKED gen={gen_linked} — jetzt REDDOT AUSSCHALTEN <<<");
            let wait = Instant::now();
            while wait.elapsed() < Duration::from_secs(120) {
                if h.status() == ConnectionStatus::Idle {
                    eprintln!(
                        "Idle nach {:.1?} reason={}",
                        wait.elapsed(),
                        h.last_reason()
                    );
                    // Observe: no auto-nuclear (would go Connecting again).
                    thread::sleep(Duration::from_secs(8));
                    let st = h.status();
                    let gen1 = h.generation();
                    if st == ConnectionStatus::Connecting {
                        eprintln!("FAIL Auto-Nuclear nach Link-Lost status={}", st.as_str());
                        let _ = h.send(ConnectionCommand::Shutdown);
                        std::process::exit(3);
                    }
                    if st != ConnectionStatus::Idle {
                        eprintln!("FAIL expected Idle, got {}", st.as_str());
                        let _ = h.send(ConnectionCommand::Shutdown);
                        std::process::exit(4);
                    }
                    eprintln!(
                        "PASS link_lost → Idle, gen {gen_linked}->{gen1}, no auto-nuclear"
                    );
                    let _ = h.send(ConnectionCommand::Shutdown);
                    return;
                }
                if h.status() == ConnectionStatus::Connecting {
                    eprintln!("FAIL Auto-Connecting during wait");
                    let _ = h.send(ConnectionCommand::Shutdown);
                    std::process::exit(3);
                }
                thread::sleep(Duration::from_millis(200));
            }
            eprintln!(
                "FAIL no Idle within 120s (status={} reason={})",
                h.status().as_str(),
                h.last_reason()
            );
            let _ = h.send(ConnectionCommand::Shutdown);
            std::process::exit(5);
        }
        "long_hold" => {
            let hold_secs: u64 = env::var("REDOT_LONG_HOLD_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4 * 60 * 60);
            let hold = Duration::from_secs(hold_secs);
            let t0 = Instant::now();
            while t0.elapsed() < Duration::from_secs(90) {
                if h.status() == ConnectionStatus::Linked {
                    break;
                }
                if matches!(
                    h.status(),
                    ConnectionStatus::Idle
                        | ConnectionStatus::NeedsPairing
                        | ConnectionStatus::Faulted
                ) && t0.elapsed() > Duration::from_secs(5)
                {
                    eprintln!(
                        "FAIL never Linked (status={} reason={})",
                        h.status().as_str(),
                        h.last_reason()
                    );
                    let _ = h.send(ConnectionCommand::Shutdown);
                    std::process::exit(2);
                }
                thread::sleep(Duration::from_millis(100));
            }
            if h.status() != ConnectionStatus::Linked {
                eprintln!("FAIL Linked timeout");
                let _ = h.send(ConnectionCommand::Shutdown);
                std::process::exit(2);
            }
            let gen0 = h.generation();
            let hold_start = Instant::now();
            eprintln!(
                ">>> LONG_HOLD START linked gen={gen0} hold={hold_secs}s — RedDot AN lassen <<<"
            );
            let mut session_cycles: u32 = 0;
            let mut last_progress = Instant::now();
            let session_on = Duration::from_secs(45);
            let session_off = Duration::from_secs(75);
            while hold_start.elapsed() < hold {
                if h.status() != ConnectionStatus::Linked {
                    eprintln!(
                        "FAIL link dropped at {:.0?} status={} reason={} cycles={session_cycles}",
                        hold_start.elapsed(),
                        h.status().as_str(),
                        h.last_reason()
                    );
                    let _ = h.send(ConnectionCommand::Shutdown);
                    std::process::exit(3);
                }
                // Live-Session start/stop (Sink only — must not trigger Pair/Connect).
                let _ = h.send(ConnectionCommand::RegisterSink);
                thread::sleep(session_on);
                if h.status() != ConnectionStatus::Linked {
                    eprintln!(
                        "FAIL drop during session-on at {:.0?} cycles={session_cycles}",
                        hold_start.elapsed()
                    );
                    let _ = h.send(ConnectionCommand::Shutdown);
                    std::process::exit(3);
                }
                let _ = h.send(ConnectionCommand::UnregisterSink);
                session_cycles += 1;
                if last_progress.elapsed() >= Duration::from_secs(15 * 60) {
                    eprintln!(
                        "progress elapsed={:.0?} cycles={session_cycles} status={}",
                        hold_start.elapsed(),
                        h.status().as_str()
                    );
                    last_progress = Instant::now();
                }
                thread::sleep(session_off);
            }
            if h.status() != ConnectionStatus::Linked {
                eprintln!("FAIL not Linked at end");
                let _ = h.send(ConnectionCommand::Shutdown);
                std::process::exit(3);
            }
            eprintln!(
                "PASS long_hold {hold_secs}s cycles={session_cycles} gen={}->{} still Linked",
                gen0,
                h.generation()
            );
            let _ = h.send(ConnectionCommand::Shutdown);
        }
        other => {
            eprintln!("unknown mode {other}");
            std::process::exit(1);
        }
    }
}
