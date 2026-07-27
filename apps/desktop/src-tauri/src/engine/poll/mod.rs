//! Poll worker — transport → raw frame → Arena ingest (commit) → UI emit.
//! Fail-closed: no UI shot unless ingest returns Accepted after COMMIT.

mod connect;
mod emit;
mod idle_wait;
mod ingest;
mod read;

use super::StandEngine;
use crate::connection::shot_latency::{
    self, FrameProvenanceTracker, PollReadTouch, TracedIncoming,
};
use crate::protocol::{Incoming, RedDotStreamParser};
use crate::transport::simulator::SimulatorControl;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub(crate) fn run_poll_loop(
    app: AppHandle,
    engine: Arc<StandEngine>,
    generation: u64,
    log_path: std::path::PathBuf,
    session_id: String,
    use_sim: bool,
    last_port: Option<String>,
    sim_control: SimulatorControl,
    stop_flag: Arc<AtomicBool>,
) {
    let still_active = || {
        !stop_flag.load(Ordering::SeqCst) && engine.generation.load(Ordering::SeqCst) == generation
    };

    let mut log = match connect::open_log(&app, &engine, generation, &log_path, use_sim) {
        Some(l) => l,
        None => return,
    };

    let mut transport = match connect::open_transport(
        &app,
        &engine,
        generation,
        &mut log,
        use_sim,
        last_port,
        sim_control.clone(),
    ) {
        Some(t) => t,
        None => return,
    };

    connect::emit_searching(&app, &engine, generation, transport.as_ref());

    let mut connected = false;
    let mut parser = RedDotStreamParser::new();
    let mut provenance = FrameProvenanceTracker::new();
    let mut last_heartbeat = Instant::now();
    const HEARTBEAT_SECS: u64 = 60;
    // DIAGNOSE-ONLY — poll iteration + wait-before-read (never changes wait duration).
    let mut poll_iteration_id: u64 = 0;
    let mut last_wait: Option<(Instant, Instant)> = None;

    while still_active() {
        // Fallback autosave heartbeat when no shots arrive (pause between groups).
        if last_heartbeat.elapsed() >= Duration::from_secs(HEARTBEAT_SECS) {
            let _ = log.touch_autosave(&session_id, None);
            last_heartbeat = Instant::now();
        }

        if !read::write_enq(&app, &engine, generation, transport.as_mut()) {
            break;
        }

        let diag = shot_latency::is_enabled();
        if diag {
            poll_iteration_id = poll_iteration_id.saturating_add(1);
        }

        // 2) Before read: was a frame already pending?
        let was_pending = diag && provenance.has_open_frame();

        let mut buf = [0u8; 256];
        let outcome =
            read::read_chunk(&app, &engine, generation, transport.as_mut(), &mut buf, diag);
        let Some(n) = outcome.n else {
            break;
        };

        // 4) Intervening read window (includes completing read) — diagnose only.
        if diag {
            let read_kind = read::classify_read_result(n);
            let read_wait_ms = outcome.timing.map(|t| t.duration_ms()).unwrap_or(0);
            provenance.accumulate_read_if_was_pending(was_pending, read_kind, read_wait_ms);
        }

        let poll_touch = if diag && n > 0 {
            let read_kind = read::classify_read_result(n);
            let timing = outcome
                .timing
                .expect("diag-on read_chunk always returns timing");
            let (wait_started, wait_returned) = match last_wait.take() {
                Some((s, r)) => (Some(s), Some(r)),
                None => (None, None),
            };
            Some(PollReadTouch {
                iteration_id: poll_iteration_id,
                wait_started,
                wait_returned,
                read_started: timing.started,
                read_returned: timing.returned,
                read_result: read_kind,
            })
        } else {
            None
        };

        if n > 0 {
            // DIAGNOSE-ONLY: RFCOMM Bridge stamps a visit; simulator has none.
            if let Some(visit) = shot_latency::take_bridge_visit() {
                for msg in provenance.feed(&mut parser, visit, &buf[..n], poll_touch) {
                    match msg {
                        TracedIncoming::Nak => {
                            emit::on_nak(
                                &app,
                                &engine,
                                generation,
                                &mut log,
                                &session_id,
                                transport.as_ref(),
                                &mut connected,
                            );
                        }
                        TracedIncoming::Shot(traced) => {
                            if ingest::handle_shot_frame(
                                &app,
                                &engine,
                                generation,
                                &mut log,
                                &session_id,
                                transport.as_mut(),
                                &mut connected,
                                &mut last_heartbeat,
                                &still_active,
                                traced.raw.clone(),
                                Some(traced),
                            ) {
                                break;
                            }
                        }
                    }
                }
            } else {
                for msg in parser.push(&buf[..n]) {
                    match msg {
                        Incoming::Nak => {
                            emit::on_nak(
                                &app,
                                &engine,
                                generation,
                                &mut log,
                                &session_id,
                                transport.as_ref(),
                                &mut connected,
                            );
                        }
                        Incoming::ShotFrame(raw) => {
                            if ingest::handle_shot_frame(
                                &app,
                                &engine,
                                generation,
                                &mut log,
                                &session_id,
                                transport.as_mut(),
                                &mut connected,
                                &mut last_heartbeat,
                                &still_active,
                                raw,
                                None,
                            ) {
                                break;
                            }
                        }
                        Incoming::Ack | Incoming::NeedMore | Incoming::Skip => {}
                    }
                }
            }
        }

        // Idle wait (80 ms): skip only on RFCOMM while parser holds incomplete STX shot.
        if idle_wait::should_call_idle_wait(
            sim_control.pending_count(),
            use_sim,
            parser.has_incomplete_shot_frame(),
        ) {
            if diag {
                let wait_started = Instant::now();
                sim_control.wait_timeout(Duration::from_millis(80));
                let wait_returned = Instant::now();
                // DIAGNOSE-ONLY: attribute wait only if provenance still has open frame.
                if provenance.has_open_frame() {
                    let wait_ms = wait_returned
                        .saturating_duration_since(wait_started)
                        .as_millis() as u64;
                    provenance.accumulate_wait_ms(wait_ms);
                }
                last_wait = Some((wait_started, wait_returned));
            } else {
                sim_control.wait_timeout(Duration::from_millis(80));
            }
        }
    }

    let _ = transport.close();
    emit::worker_ended(&app, &engine, generation, transport.as_ref());
}
