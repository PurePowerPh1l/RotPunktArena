//! Series completion + synthetic inject / fire helpers.

use super::{ConnectionUpdate, LiveState, SeriesCompletePayload, StandEngine, UiShot};
use crate::transport::ConnectionStatus;
use tauri::{AppHandle, Emitter};

impl StandEngine {
    #[allow(dead_code)]
    pub fn queue_sim_shot(
        &self,
        value_ascii: String,
        distance_ascii: String,
        x_ascii: String,
        y_ascii: String,
    ) -> Result<(), String> {
        self.sim_control
            .queue_synthetic(&value_ascii, &distance_ascii, &x_ascii, &y_ascii)
    }

    /// Direct Arena ingest (same path as hardware) — reliable for click-to-shoot / Dev tests.
    /// Does not depend on the poll worker reading the simulator queue.
    pub fn inject_synthetic_shot(
        &self,
        app: &AppHandle,
        value_ascii: &str,
        distance_ascii: &str,
        x_ascii: &str,
        y_ascii: &str,
    ) -> Result<LiveState, String> {
        use crate::arena::IngestOutcome;
        use crate::protocol::{build_synthetic_shot_frame, stamp_frame_nonce};

        {
            let g = self.inner.lock();
            if g.series_complete {
                return Err("Serie bereits beendet".into());
            }
            if let Some(max) = g.max_shots {
                if g.shots.len() as i64 >= max {
                    return Err(format!("Maximal {max} Schüsse erreicht"));
                }
            }
        }

        let session_id = self
            .snapshot()
            .session
            .filter(|s| s.ended_at.is_none())
            .ok_or_else(|| "Keine offene Session — zuerst starten".to_string())?
            .id;

        let mut frame = build_synthetic_shot_frame(value_ascii, distance_ascii, x_ascii, y_ascii)?;
        stamp_frame_nonce(&mut frame);

        let accepted = self.with_db_mut(|db| {
            match db.ingest_raw_frame(&session_id, &frame, "dev", None)? {
                IngestOutcome::Accepted(a) => {
                    db.try_maybe_snapshot_after_shot(&session_id, a.shot_index, a.session_sequence);
                    Ok(a)
                }
                IngestOutcome::Duplicate { .. } => {
                    Err("Unerwartetes Duplikat — nochmal versuchen".into())
                }
                IngestOutcome::ParseFailed { error, .. } => Err(format!("Parse: {error}")),
                IngestOutcome::LimitReached {
                    max_shots,
                    current_shots,
                } => Err(format!(
                    "Maximal {max_shots} Schüsse erreicht ({current_shots}/{max_shots})"
                )),
                IngestOutcome::SessionInactive { .. } => {
                    Err("Keine offene Session — zuerst starten".into())
                }
            }
        })?;

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
        self.apply_shot(ui.clone());
        let _ = app.emit("shot", ui);
        self.finish_series_if_needed(app, accepted.shot_index as i64);
        Ok(self.snapshot())
    }

    pub fn fire_aim_shot(&self, app: &AppHandle, x: f64, y: f64) -> Result<LiveState, String> {
        let (value, dist, x_ascii, y_ascii) = crate::protocol::aim_coords_to_ascii(x, y);
        self.inject_synthetic_shot(app, &value, &dist, &x_ascii, &y_ascii)
    }

    /// After the last competition shot: close session, mark entry done, notify UI.
    pub fn finish_series_if_needed(&self, app: &AppHandle, shot_index: i64) {
        let (max, shooter, total) = {
            let g = self.inner.lock();
            (
                g.max_shots,
                g.session
                    .as_ref()
                    .map(|s| s.shooter_name.clone())
                    .unwrap_or_default(),
                g.series_total,
            )
        };
        let Some(max) = max else {
            return;
        };
        if shot_index < max {
            return;
        }
        {
            let g = self.inner.lock();
            if g.series_complete {
                return;
            }
        }

        let _ = self.end_session();
        {
            let mut g = self.inner.lock();
            g.series_complete = true;
        }
        let payload = SeriesCompletePayload {
            max_shots: max,
            shot_count: shot_index,
            series_total: total,
            shooter_name: shooter,
        };
        let _ = app.emit("series_complete", payload.clone());
        let _ = app.emit(
            "connection",
            ConnectionUpdate {
                status: ConnectionStatus::Disconnected,
                transport: self.snapshot().transport,
                port: None,
                detail: Some(format!(
                    "Serie beendet — {shot_index}/{max} Schüsse · {}",
                    payload.shooter_name
                )),
            },
        );
    }

    pub fn set_auto_fire(&self, on: bool) {
        self.sim_control.set_auto_fire(on);
        self.inner.lock().auto_fire = on;
    }

    pub fn list_ports(&self) -> Vec<String> {
        crate::transport::list_serial_ports()
    }

    pub fn auto_detect(&self) -> Option<String> {
        let last = self.log.lock().get_setting("last_port").ok().flatten();
        crate::transport::auto_detect(last.as_deref())
    }
}
