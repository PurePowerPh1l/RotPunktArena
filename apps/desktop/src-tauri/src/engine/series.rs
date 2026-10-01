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
        self.require_simulator_session()?;
        self.sim_control
            .queue_synthetic(&value_ascii, &distance_ascii, &x_ascii, &y_ascii)
    }

    /// Direct Arena ingest (same path as hardware) — reliable for click-to-shoot / Dev tests.
    /// Does not depend on the poll worker reading the simulator queue.
    pub fn inject_synthetic_shot<R: tauri::Runtime>(
        &self,
        app: &AppHandle<R>,
        value_ascii: &str,
        distance_ascii: &str,
        x_ascii: &str,
        y_ascii: &str,
    ) -> Result<LiveState, String> {
        use crate::arena::IngestOutcome;
        use crate::protocol::{build_synthetic_shot_frame, stamp_frame_nonce};

        let transition = self.ingest_gate.lock();
        self.require_simulator_session()?;

        {
            let g = self.inner.lock();
            if g.series_complete {
                return Err("Serie bereits beendet".into());
            }
            // Probe phase: unlimited shots, limit applies to the scored series only.
            if !g.probe_active {
                if let Some(max) = g.max_shots {
                    if g.shots.len() as i64 >= max {
                        return Err(format!("Maximal {max} Schüsse erreicht"));
                    }
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
            match db.ingest_raw_frame(&session_id, &frame, "simulator", None)? {
                IngestOutcome::Accepted(a) => {
                    db.spawn_maybe_snapshot_after_shot(
                        &session_id,
                        a.shot_index,
                        a.session_sequence,
                    );
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
        drop(transition);
        self.finish_series_if_needed(app, accepted.shot_index as i64)?;
        Ok(self.snapshot())
    }

    pub fn fire_aim_shot<R: tauri::Runtime>(&self, app: &AppHandle<R>, x: f64, y: f64) -> Result<LiveState, String> {
        let (value, dist, x_ascii, y_ascii) = crate::protocol::aim_coords_to_ascii(x, y);
        self.inject_synthetic_shot(app, &value, &dist, &x_ascii, &y_ascii)
    }

    /// After the last competition shot: close session, mark entry done, notify UI.
    /// No-op during the probe phase (Probeschüsse never finish the series).
    pub fn finish_series_if_needed<R: tauri::Runtime>(&self, app: &AppHandle<R>, shot_index: i64) -> Result<(), String> {
        let Some(payload) = self.complete_series_if_needed(shot_index)? else {
            return Ok(());
        };
        let _ = app.emit("series_complete", payload.clone());
        let _ = app.emit(
            "connection",
            ConnectionUpdate {
                status: ConnectionStatus::Disconnected,
                transport: self.snapshot().transport,
                port: None,
                detail: Some(format!(
                    "Serie beendet — {}/{} Schüsse · {}",
                    payload.shot_count, payload.max_shots, payload.shooter_name
                )),
            },
        );
        Ok(())
    }

    /// Shared by recovery and automatic completion. Never snapshot under `inner`.
    pub(super) fn close_completed_series(&self) -> Result<LiveState, String> {
        self.end_session()?;
        {
            self.inner.lock().series_complete = true;
        }
        Ok(self.snapshot())
    }

    fn complete_series_if_needed(
        &self,
        shot_index: i64,
    ) -> Result<Option<SeriesCompletePayload>, String> {
        let (max, shooter, total) = {
            let g = self.inner.lock();
            if g.probe_active {
                return Ok(None);
            }
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
            return Ok(None);
        };
        if shot_index < max {
            return Ok(None);
        }
        {
            let g = self.inner.lock();
            if g.series_complete {
                return Ok(None);
            }
        }

        self.close_completed_series()?;
        let payload = SeriesCompletePayload {
            max_shots: max,
            shot_count: shot_index,
            series_total: total,
            shooter_name: shooter,
        };
        Ok(Some(payload))
    }

    pub fn set_auto_fire(&self, on: bool) -> Result<(), String> {
        self.require_simulator_session()?;
        self.sim_control.set_auto_fire(on);
        self.inner.lock().auto_fire = on;
        Ok(())
    }

    fn require_simulator_session(&self) -> Result<(), String> {
        let state = self.inner.lock();
        let session = state.session.as_ref().filter(|s| s.ended_at.is_none())
            .ok_or("Keine offene Simulatorsession")?;
        if state.transport != crate::transport::TransportKind::Simulator || !session.simulated || session.competition_id.is_some() {
            return Err("Synthetische Schüsse sind ausschließlich im Simulatortraining erlaubt".into());
        }
        Ok(())
    }

    pub fn list_ports(&self) -> Vec<String> {
        crate::transport::list_serial_ports()
    }

    pub fn auto_detect(&self) -> Option<String> {
        let last = self.log.lock().get_setting("last_port").ok().flatten();
        crate::transport::auto_detect(last.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn synthetic_ingest_requires_persisted_simulator_training_and_stays_out_of_statistics() {
        let engine = engine_fixture();
        let app = tauri::test::mock_app();
        assert!(engine.fire_aim_shot(app.handle(), 1.0, 2.0).is_err());
        let session_id = engine.snapshot().session.unwrap().id;
        engine.with_db(|db| db.conn.execute("UPDATE sessions SET simulated=1 WHERE id=?1", [&session_id]).unwrap());
        engine.inner.lock().session.as_mut().unwrap().simulated = true;
        engine.inner.lock().transport = crate::transport::TransportKind::Rfcomm;
        assert!(engine.fire_aim_shot(app.handle(), 1.0, 2.0).is_err());
        engine.inner.lock().transport = crate::transport::TransportKind::Simulator;
        engine.inner.lock().session.as_mut().unwrap().competition_id = Some("fixture".into());
        assert!(engine.fire_aim_shot(app.handle(), 1.0, 2.0).is_err());
        engine.inner.lock().session.as_mut().unwrap().competition_id = None;
        engine.fire_aim_shot(app.handle(), 1.0, 2.0).unwrap();
        assert_eq!(engine.with_db(|db| db.count_session_shots(&session_id)).unwrap(), 1);
        let ended = engine.end_session().unwrap();
        assert_eq!(ended.training_save.unwrap().reason, "simulated");
        assert!(engine.fire_aim_shot(app.handle(), 1.0, 2.0).is_err());
        assert_eq!(engine.with_db(|db| db.list_saved_training_sessions(80, None, None)).unwrap().len(), 0);
    }

    fn engine_fixture() -> Arc<StandEngine> {
        let db = Database::open_in_memory().unwrap();
        let (session, _, _) = db
            .start_live_session("Fixture", None, None, None, Some(5))
            .unwrap();
        let engine = Arc::new(StandEngine::new(db));
        {
            let mut state = engine.inner.lock();
            state.session = Some(session);
            state.max_shots = Some(5);
            state.series_total = 50.0;
        }
        engine
    }

    #[test]
    fn completed_recovery_releases_guard_before_snapshot() {
        let engine = engine_fixture();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || sender.send(engine.close_completed_series()).unwrap());
        let state = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("recovery must not deadlock")
            .unwrap();
        assert!(state.series_complete);
        assert!(state.session.unwrap().ended_at.is_some());
    }

    #[test]
    fn failed_completion_stays_open_without_success_and_can_retry() {
        let engine = engine_fixture();
        engine.with_db(|db| {
            db.conn.execute_batch(
            "CREATE TRIGGER fail_end BEFORE INSERT ON events WHEN NEW.kind = 'session_ended'
             BEGIN SELECT RAISE(ABORT, 'injected completion failure'); END;"
        ).unwrap()
        });
        assert!(engine.complete_series_if_needed(5).is_err());
        let state = engine.snapshot();
        assert!(!state.series_complete);
        let session = state.session.unwrap();
        assert!(session.ended_at.is_none());
        engine.with_db(|db| {
            assert!(db
                .get_session(&session.id)
                .unwrap()
                .unwrap()
                .ended_at
                .is_none());
            assert_eq!(
                db.count_events_kind(crate::db::event_kind::SESSION_ENDED)
                    .unwrap(),
                0
            );
            db.conn.execute_batch("DROP TRIGGER fail_end").unwrap();
        });
        let payload = engine.complete_series_if_needed(5).unwrap().unwrap();
        assert_eq!(payload.shot_count, 5);
        assert!(engine.snapshot().series_complete);
        assert!(engine.complete_series_if_needed(5).unwrap().is_none());
        assert_eq!(
            engine
                .with_db(|db| db.count_events_kind(crate::db::event_kind::SESSION_ENDED))
                .unwrap(),
            1
        );
    }

    #[test]
    fn completion_ignores_probe_unlimited_and_incomplete_series() {
        let engine = engine_fixture();
        assert!(engine.complete_series_if_needed(4).unwrap().is_none());
        engine.inner.lock().probe_active = true;
        assert!(engine.complete_series_if_needed(5).unwrap().is_none());
        {
            let mut state = engine.inner.lock();
            state.probe_active = false;
            state.max_shots = None;
        }
        assert!(engine.complete_series_if_needed(100).unwrap().is_none());
        assert!(engine.snapshot().session.unwrap().ended_at.is_none());
    }
}
