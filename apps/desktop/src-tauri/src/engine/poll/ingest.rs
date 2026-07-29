//! Arena ingest of a raw shot frame + UI emit on Accepted.

use crate::arena::IngestOutcome;
use crate::connection::shot_latency::{self, TracedShotFrame};
use crate::db::Database;
use crate::protocol::encode_ack;
use crate::transport::{ConnectionStatus, Transport};
use super::super::{emit_conn, ConnectionUpdate, StandEngine, UiShot};
use super::emit;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// Handle one ShotFrame. Returns `true` if the caller should break out of the message for-loop
/// (generation stale after Accepted).
///
/// `latency_trace`: DIAGNOSE-ONLY provenance from Bridge/Owner; never changes accept/emit.
pub(super) fn handle_shot_frame(
    app: &AppHandle,
    engine: &Arc<StandEngine>,
    generation: u64,
    log: &mut Database,
    session_id: &str,
    transport: &mut dyn Transport,
    connected: &mut bool,
    last_heartbeat: &mut std::time::Instant,
    still_active: &dyn Fn() -> bool,
    raw: Vec<u8>,
    latency_trace: Option<TracedShotFrame>,
) -> bool {
    emit::ensure_connected(app, engine, generation, transport, connected);

    // Fast poll gate (not sole authority — Arena re-checks ended_at in TX).
    let poll_ok = still_active() && engine.poll_session_accepting(session_id);
    if !poll_ok {
        // ACK like a processed frame so the device does not resend into a later series.
        let _ = transport.write_all(&encode_ack());
        return !still_active();
    }

    let ingest_started = Instant::now();
    let mode = engine.session_mode_label();
    let result = log.ingest_raw_frame(session_id, &raw, "device", None);

    // ACK only after the persist attempt succeeded (any Ok outcome, incl.
    // Duplicate — device may resend). On persist Err we intentionally do NOT
    // ACK so the device retransmits and the shot gets another chance.
    if result.is_ok() {
        let _ = transport.write_all(&encode_ack());
    }

    match result {
        Ok(IngestOutcome::Accepted(accepted)) => {
            if !still_active() {
                return true;
            }
            *last_heartbeat = Instant::now();
            let ui = UiShot {
                shot_index: accepted.shot_index as u32,
                value_raw: accepted.value_raw,
                distance_raw: accepted.distance_raw,
                x: accepted.x,
                y: accepted.y,
                value_display: accepted.score,
                distance_display: accepted.distance_raw as f64 / 10.0,
                series_total: accepted.series_total,
                series_teiler_total: accepted.series_teiler_total,
            };
            // Emit only after successful commit (Accepted).
            let sqlite_committed = shot_latency::take_sqlite_committed_at();
            if engine.apply_shot(ui.clone()) {
                let shot_event_emitted = Instant::now();
                let _ = app.emit("shot", ui);
                // DIAGNOSE-ONLY — best-effort; never gates Accepted/emit.
                if let Some(traced) = latency_trace {
                    shot_latency::append_accepted_shot(
                        session_id,
                        mode,
                        &traced,
                        ingest_started,
                        sqlite_committed.unwrap_or(shot_event_emitted),
                        shot_event_emitted,
                    );
                }
            }
            engine.finish_series_if_needed(app, i64::from(accepted.shot_index));
            // Hybrid snapshot on a background thread — the poll loop keeps
            // draining the sink instead of blocking on VACUUM I/O.
            log.spawn_maybe_snapshot_after_shot(
                session_id,
                accepted.shot_index,
                accepted.session_sequence,
            );
            false
        }
        Ok(IngestOutcome::Duplicate { .. }) => {
            // Idempotent — no UI double-count.
            false
        }
        Ok(IngestOutcome::SessionInactive { .. }) => {
            // Ended / mismatched session — already ACKed; no UI.
            false
        }
        Ok(IngestOutcome::LimitReached {
            max_shots,
            current_shots,
        }) => {
            engine.finish_series_if_needed(app, current_shots);
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Disconnected,
                    transport: transport.kind(),
                    port: Some(transport.name().to_string()),
                    detail: Some(format!(
                        "Serie beendet — {current_shots}/{max_shots} Schüsse"
                    )),
                },
            );
            false
        }
        Ok(IngestOutcome::ParseFailed { error, .. }) => {
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Connected,
                    transport: transport.kind(),
                    port: Some(transport.name().to_string()),
                    detail: Some(format!("Frame-Parse: {error}")),
                },
            );
            false
        }
        Err(e) => {
            // Fail-closed: persist failed → no UI shot.
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Connected,
                    transport: transport.kind(),
                    port: Some(transport.name().to_string()),
                    detail: Some(format!("Persistenz fehlgeschlagen: {e}")),
                },
            );
            false
        }
    }
}
