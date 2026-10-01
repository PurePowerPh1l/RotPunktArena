//! Session start / resume / end / reset / close.

use super::poll;
use super::{ConnectionUpdate, LiveState, StandEngine, StartSessionArgs, UiShot};
use crate::transport::{ConnectionStatus, TransportKind};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter};

impl StandEngine {
    pub fn is_running(&self) -> bool {
        let g = self.inner.lock();
        matches!(
            g.status,
            ConnectionStatus::Searching | ConnectionStatus::Connected
        ) && g.session.as_ref().is_some_and(|s| s.ended_at.is_none())
    }

    /// Idempotent: if already running, return current snapshot without restart.
    pub fn start_session(
        self: &Arc<Self>,
        app: AppHandle,
        args: StartSessionArgs,
    ) -> Result<LiveState, String> {
        let _lifecycle = self.lifecycle_gate.lock();
        if self.is_running() {
            return Ok(self.snapshot());
        }

        // A dead poll worker leaves the previous session open in the DB
        // (status Disconnected, ended_at IS NULL). Close it via the normal
        // end path so it does not linger as a recovery orphan next to the
        // new session.
        let stale_open = {
            let g = self.inner.lock();
            g.session.as_ref().is_some_and(|s| s.ended_at.is_none())
        };
        if stale_open {
            self.end_session()?;
        }

        self.stop_worker();

        let name = if args.shooter_name.trim().is_empty() {
            "Schütze".to_string()
        } else {
            args.shooter_name.trim().to_string()
        };

        let endless = args.competition_id.is_none() && args.endless;
        let training_limit = if endless {
            None
        } else {
            Some(crate::db::normalize_training_series_shots(
                self.inner.lock().training_series_shots,
            ))
        };
        let last_port = self.log.lock().get_setting("last_port")?;
        let (session, max_shots, probe) = self.with_db(|db| {
            db.start_live_session_with_source(
                &name,
                args.competition_id.as_deref(),
                args.entry_id.as_deref(),
                args.person_id.as_deref(),
                training_limit,
                args.use_simulator,
            )
        })?;

        {
            let mut g = self.inner.lock();
            g.session = Some(session.clone());
            g.shots.clear();
            g.series_total = 0.0;
            g.series_teiler_total = 0.0;
            g.status = ConnectionStatus::Searching;
            g.transport = if args.use_simulator {
                TransportKind::Simulator
            } else {
                TransportKind::Rfcomm
            };
            g.port = None;
            g.auto_fire = false;
            g.max_shots = max_shots;
            g.series_complete = false;
            g.endless_mode = endless;
            g.probe_active = probe;
            // Fresh session — clear prior end-of-series save outcome
            // (reset_training_series re-applies it after start).
            g.last_training_save = None;
        }
        self.sim_control.set_auto_fire(false);
        self.emit_connection(&app);

        self.stop.store(false, Ordering::SeqCst);
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let stop_flag = self.stop.clone();
        let sim_control = self.sim_control.clone();
        let engine_log_path = self.log.lock().path().to_path_buf();
        let session_id = session.id.clone();
        let use_sim = args.use_simulator;
        let status_app = app.clone();
        let engine = Arc::clone(self);

        let handle = thread::Builder::new()
            .name("reddot-poll".into())
            .spawn(move || {
                poll::run_poll_loop(
                    status_app,
                    engine,
                    gen,
                    engine_log_path,
                    session_id,
                    use_sim,
                    last_port,
                    sim_control,
                    stop_flag,
                );
            })
            .map_err(|e| e.to_string())?;

        *self.worker.lock() = Some(handle);
        Ok(self.snapshot())
    }

    /// Resume an interrupted session: no new INSERT; rehydrate shots; start poll on same id.
    pub fn resume_session(
        self: &Arc<Self>,
        app: AppHandle,
        session_id: &str,
        use_simulator: bool,
    ) -> Result<LiveState, String> {
        let _lifecycle = self.lifecycle_gate.lock();
        if self.is_running() {
            return Err("Es läuft bereits eine Session — zuerst beenden".into());
        }

        self.stop_worker();

        let session = self
            .with_db(|db| db.get_session(session_id))?
            .ok_or_else(|| "Session nicht gefunden".to_string())?;
        if session.ended_at.is_some() {
            return Err("Session ist bereits beendet".into());
        }
        if session.simulated != use_simulator {
            return Err("Recovery muss denselben Hardware-/Simulatormodus wie die gespeicherte Session verwenden".into());
        }
        let open = self.with_db(|db| db.list_unclean_sessions())?;
        if !open.iter().any(|id| id == session_id) {
            return Err("Session ist nicht zur Wiederaufnahme vorgesehen".into());
        }

        self.with_db_mut(|db| db.mark_session_recovered(session_id))?;

        // Resume in the persisted phase: probe rehydrates probe shots,
        // match rehydrates only scored shots.
        let probe =
            self.with_db(|db| db.get_session_phase(session_id))? == crate::db::session_phase::PROBE;
        let classification = if probe {
            crate::db::shot_classification::PROBE
        } else {
            crate::db::shot_classification::SCORED
        };
        let stored = self.with_db(|db| db.load_session_ui_shots(session_id, classification))?;
        let shots: Vec<UiShot> = stored
            .into_iter()
            .map(|s| UiShot {
                shot_index: s.shot_index,
                value_raw: s.value_raw,
                distance_raw: s.distance_raw,
                x: s.x,
                y: s.y,
                value_display: s.value_display,
                distance_display: s.distance_display,
                series_total: s.series_total,
                series_teiler_total: s.series_teiler_total,
            })
            .collect();
        let series_total = shots.last().map(|s| s.series_total).unwrap_or(0.0);
        let series_teiler_total = shots.last().map(|s| s.series_teiler_total).unwrap_or(0.0);

        if let Some(ref entry_id) = session.entry_id {
            self.with_db(|db| db.reactivate_entry_for_resume(entry_id))?;
        }

        // Interrupted sessions resume as normal series (endless is not persisted).
        // Prefer the limit already stored on the session row.
        let pref = {
            let g = self.inner.lock();
            crate::db::normalize_training_series_shots(g.training_series_shots)
        };
        let max_shots = match &session.competition_id {
            Some(cid) => {
                self.with_db(|db| db.effective_max_shots(cid, session.entry_id.as_deref()))?
            }
            None => {
                let stored = self.with_db(|db| db.get_session_max_shots(session_id))?;
                Some(
                    stored
                        .filter(|&n| n > 0)
                        .map(crate::db::normalize_training_series_shots)
                        .unwrap_or(pref),
                )
            }
        };
        self.with_db_mut(|db| db.set_session_max_shots(session_id, max_shots))?;
        let series_complete = !probe && max_shots.is_some_and(|m| shots.len() as i64 >= m);

        {
            let mut g = self.inner.lock();
            g.session = Some(session.clone());
            g.shots = shots;
            g.series_total = series_total;
            g.series_teiler_total = series_teiler_total;
            g.status = ConnectionStatus::Searching;
            g.transport = if use_simulator {
                TransportKind::Simulator
            } else {
                TransportKind::Rfcomm
            };
            g.port = None;
            g.auto_fire = false;
            g.max_shots = max_shots;
            g.series_complete = false;
            g.endless_mode = false;
            g.probe_active = probe;
            g.last_training_save = None;
        }
        self.sim_control.set_auto_fire(false);
        self.emit_connection(&app);

        if series_complete {
            // Already at limit — do not restart poll; leave disconnected after close.
            return self.close_completed_series();
        }

        let last_port = self.log.lock().get_setting("last_port")?;
        self.stop.store(false, Ordering::SeqCst);
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let stop_flag = self.stop.clone();
        let sim_control = self.sim_control.clone();
        let engine_log_path = self.log.lock().path().to_path_buf();
        let sid = session.id.clone();
        let status_app = app.clone();
        let engine = Arc::clone(self);

        let handle = thread::Builder::new()
            .name("reddot-poll".into())
            .spawn(move || {
                poll::run_poll_loop(
                    status_app,
                    engine,
                    gen,
                    engine_log_path,
                    sid,
                    use_simulator,
                    last_port,
                    sim_control,
                    stop_flag,
                );
            })
            .map_err(|e| e.to_string())?;

        *self.worker.lock() = Some(handle);
        Ok(self.snapshot())
    }

    /// End the probe phase ("Wertung beginnen"): persist `match` phase,
    /// clear probe shots from the live UI, and start the scored series.
    pub fn finish_probe(&self, app: &AppHandle) -> Result<LiveState, String> {
        self.finish_probe_transition(|probe_shots, shooter| {
            if let Err(error) = app.emit("probe_finished",
                serde_json::json!({ "probeShots": probe_shots, "shooterName": shooter })) {
                eprintln!("Probewechsel konnte nicht angezeigt werden: {error}");
            }
        })
    }

    fn finish_probe_transition(&self, notify: impl FnOnce(i64, String)) -> Result<LiveState, String> {
        let _transition = self.ingest_gate.lock();
        let (session_id, probe_shots, shooter) = {
            let g = self.inner.lock();
            let s = g
                .session
                .as_ref()
                .filter(|s| s.ended_at.is_none())
                .ok_or_else(|| "Keine offene Session".to_string())?;
            if !g.probe_active {
                return Err("Keine Probephase aktiv".into());
            }
            (s.id.clone(), g.shots.len() as i64, s.shooter_name.clone())
        };

        self.with_db(|db| db.finish_probe_phase(&session_id, probe_shots))?;

        {
            let mut g = self.inner.lock();
            g.probe_active = false;
            g.shots.clear();
            g.series_total = 0.0;
            g.series_teiler_total = 0.0;
            g.series_complete = false;
        }
        notify(probe_shots, shooter);
        Ok(self.snapshot())
    }

    /// Close interrupted session (Recovery Gate) or current live session via shared DB path.
    pub fn close_interrupted_session(&self, session_id: &str) -> Result<LiveState, String> {
        let current_id = {
            let g = self.inner.lock();
            g.session.as_ref().map(|s| s.id.clone())
        };
        if current_id.as_deref() == Some(session_id) {
            return self.end_session();
        }
        self.with_db_mut(|db| db.close_interrupted_session(session_id))?;
        Ok(self.snapshot())
    }

    /// Signal stop; retain handles for the maintenance shutdown barrier.
    pub fn stop_worker(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.sim_control.notify();
        if let Some(h) = self.worker.lock().take() {
            self.retired_workers.lock().push(h);
        }
        // A worker can call stop itself. Only join handles already finished here.
        let mut retired = self.retired_workers.lock();
        let mut index = 0;
        while index < retired.len() {
            if retired[index].is_finished() {
                if retired.swap_remove(index).join().is_err() {
                    eprintln!("Beendeter Poll-Worker ist fehlgeschlagen");
                }
            } else { index += 1; }
        }
        drop(retired);
        let mut g = self.inner.lock();
        g.status = ConnectionStatus::Disconnected;
        g.auto_fire = false;
    }

    pub fn shutdown(&self) {
        let _lifecycle = self.lifecycle_gate.lock();
        self.stop_worker();
        for worker in std::mem::take(&mut *self.retired_workers.lock()) {
            if worker.join().is_err() { eprintln!("Poll-Worker beim Shutdown fehlgeschlagen"); }
        }
    }

    /// Idempotent: safe if already stopped / no open session.
    /// Training history uses [`Database::maybe_save_training_history`] (DB count + min shots).
    /// Endless training never sets `training_saved` (no history / stats).
    pub fn end_session(&self) -> Result<LiveState, String> {
        let _transition = self.ingest_gate.lock();
        let session_meta = {
            let g = self.inner.lock();
            g.session.as_ref().map(|s| {
                (
                    s.id.clone(),
                    s.ended_at.clone(),
                    s.entry_id.clone(),
                    s.competition_id.clone(),
                    g.endless_mode,
                    g.shots.len() as i64,
                )
            })
        };

        self.stop_worker();

        if let Some((id, ended_at, entry_id, competition_id, endless, shot_count)) = session_meta {
            if ended_at.is_none() {
                let save_info = self.with_db(|db| db.finish_live_session(
                    &id, entry_id.as_deref(), competition_id.is_none(), endless, shot_count,
                ))?;
                let mut g = self.inner.lock();
                if let Some(s) = g.session.as_mut() {
                    s.ended_at = Some(chrono::Utc::now().to_rfc3339());
                }
                g.last_training_save = Some(save_info);
            }
        }
        Ok(self.snapshot())
    }

    /// Optional early save: end (if still open) and ensure training history flag.
    /// Prefer `end_session` — it applies the same smart save policy.
    pub fn save_training_session(&self) -> Result<LiveState, String> {
        let (id, competition_id, already_ended, endless) = {
            let g = self.inner.lock();
            let s = g
                .session
                .as_ref()
                .ok_or_else(|| "Keine Session".to_string())?;
            (
                s.id.clone(),
                s.competition_id.clone(),
                s.ended_at.is_some(),
                g.endless_mode,
            )
        };
        if competition_id.is_some() {
            return Err("Speichern nur im Training".into());
        }
        if endless {
            return Err("Endlosmodus wird nicht in der Statistik gespeichert".into());
        }
        if !already_ended {
            return self.end_session();
        }
        let info = self.with_db(|db| db.maybe_save_training_history(&id, true))?;
        {
            let mut g = self.inner.lock();
            g.last_training_save = Some(info.clone());
        }
        if !info.saved {
            return Err(match info.reason.as_str() {
                "empty" => "Keine Schüsse zum Speichern".into(),
                "too_short" => format!(
                    "Zu wenige Schüsse ({}/{}) — Serie erscheint nicht in der Statistik",
                    info.shot_count, info.min_shots
                ),
                other => format!("Nicht gespeichert ({other})"),
            });
        }
        Ok(self.snapshot())
    }

    /// Toggle training endless mode (preference + live session limit).
    pub fn set_training_endless(
        &self,
        app: &AppHandle,
        endless: bool,
    ) -> Result<LiveState, String> {
        let (finish_at, persist) = {
            let mut g = self.inner.lock();
            if g.session
                .as_ref()
                .is_some_and(|s| s.competition_id.is_some())
            {
                return Err("Endlosmodus nur im Training".into());
            }
            g.endless_mode = endless;
            let open =
                g.session.as_ref().is_some_and(|s| s.ended_at.is_none()) && !g.series_complete;
            if !open {
                (None, None)
            } else {
                let session_id = g.session.as_ref().map(|s| s.id.clone());
                if endless {
                    g.max_shots = None;
                    (None, session_id.map(|id| (id, None)))
                } else {
                    let max = crate::db::normalize_training_series_shots(g.training_series_shots);
                    g.max_shots = Some(max);
                    let n = g.shots.len() as i64;
                    let finish = if n >= max { Some(n) } else { None };
                    (finish, session_id.map(|id| (id, Some(max))))
                }
            }
        };
        // Keep the persisted per-session limit in sync so Arena ingest
        // enforces the same cap as the live engine.
        if let Some((session_id, max)) = persist {
            self.with_db_mut(|db| db.set_session_max_shots(&session_id, max))?;
        }
        if let Some(shot_index) = finish_at {
            self.finish_series_if_needed(app, shot_index)?;
        }
        Ok(self.snapshot())
    }

    /// Remember preferred training series length (does not mutate an open session).
    pub fn set_training_series_shots_pref(&self, shots: i64) {
        let n = crate::db::normalize_training_series_shots(shots);
        let mut g = self.inner.lock();
        g.training_series_shots = n;
    }

    /// Set training series length preference and apply to an open non-endless session.
    pub fn set_training_series_shots(
        &self,
        app: &AppHandle,
        shots: i64,
    ) -> Result<LiveState, String> {
        let n = crate::db::normalize_training_series_shots(shots);
        let (finish_at, persist) = {
            let mut g = self.inner.lock();
            if g.session
                .as_ref()
                .is_some_and(|s| s.competition_id.is_some())
            {
                return Err("Schusszahl nur im Training änderbar".into());
            }
            g.training_series_shots = n;
            let open =
                g.session.as_ref().is_some_and(|s| s.ended_at.is_none()) && !g.series_complete;
            if !open || g.endless_mode {
                (None, None)
            } else {
                let session_id = g.session.as_ref().map(|s| s.id.clone());
                g.max_shots = Some(n);
                let count = g.shots.len() as i64;
                let finish = if count >= n { Some(count) } else { None };
                (finish, session_id.map(|id| (id, Some(n))))
            }
        };
        if let Some((session_id, max)) = persist {
            self.with_db_mut(|db| db.set_session_max_shots(&session_id, max))?;
        }
        if let Some(shot_index) = finish_at {
            self.finish_series_if_needed(app, shot_index)?;
        }
        Ok(self.snapshot())
    }

    /// End current training series (smart-saved if enough shots) and open a fresh session.
    pub fn reset_training_series(self: &Arc<Self>, app: AppHandle) -> Result<LiveState, String> {
        let (name, use_simulator, competition_id, person_id, endless) = {
            let g = self.inner.lock();
            let s = g
                .session
                .as_ref()
                .ok_or_else(|| "Keine Session — zuerst Training starten".to_string())?;
            (
                s.shooter_name.clone(),
                matches!(g.transport, TransportKind::Simulator),
                s.competition_id.clone(),
                s.person_id.clone(),
                g.endless_mode,
            )
        };
        if competition_id.is_some() {
            return Err("Zurücksetzen nur im Training".into());
        }
        let ended = self.end_session()?;
        let save = ended.training_save.clone();
        // Force start even if a race left status non-disconnected
        {
            let mut g = self.inner.lock();
            g.status = ConnectionStatus::Disconnected;
            g.session = None;
            g.shots.clear();
            g.series_total = 0.0;
            g.series_teiler_total = 0.0;
            g.series_complete = false;
            g.max_shots = None;
            g.endless_mode = endless;
            // Preserve outcome through start_session (which does not clear it).
            g.last_training_save = save.clone();
        }
        let state = self.start_session(
            app,
            StartSessionArgs {
                shooter_name: name,
                use_simulator,
                competition_id: None,
                entry_id: None,
                person_id,
                endless,
            },
        )?;
        if save.is_some() {
            let mut g = self.inner.lock();
            g.last_training_save = save;
        }
        Ok(state)
    }

    fn emit_connection(&self, app: &AppHandle) {
        let g = self.inner.lock();
        let _ = app.emit(
            "connection",
            ConnectionUpdate {
                status: g.status,
                transport: g.transport,
                port: g.port.clone(),
                detail: None,
            },
        );
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    use crate::db::Database;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn ingest_waits_for_probe_commit_projection_reset_and_notification() {
        let mut db = Database::open_in_memory().unwrap();
        let session = db.start_session("Fixture", None, None, None).unwrap();
        db.set_session_phase(&session.id, "probe").unwrap();
        let engine = Arc::new(StandEngine::new(db));
        {
            let mut state = engine.inner.lock();
            state.session = Some(session.clone());
            state.probe_active = true;
        }
        let (notified, notification) = mpsc::channel();
        let (release, wait_release) = mpsc::channel();
        let transition_engine = engine.clone();
        let transition = std::thread::spawn(move || transition_engine.finish_probe_transition(|_, _| {
            notified.send(()).unwrap();
            wait_release.recv().unwrap();
        }).unwrap());
        notification.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(engine.ingest_gate.try_lock().is_none());
        let ingest_engine = engine.clone();
        let (ready, wait_ready) = mpsc::channel();
        let ingest = std::thread::spawn(move || {
            ready.send(()).unwrap();
            let _gate = ingest_engine.ingest_gate.lock();
            let frame = crate::protocol::build_synthetic_shot_frame("10.0", "001.00", "00001", "00002").unwrap();
            let crate::arena::IngestOutcome::Accepted(a) = ingest_engine.with_db_mut(|db|
                db.ingest_raw_frame(&session.id, &frame, "test", None)).unwrap() else { panic!("shot rejected") };
            assert_eq!(a.shot_index, 1);
            ingest_engine.apply_shot(UiShot {
                shot_index: 1, value_raw: a.value_raw, distance_raw: a.distance_raw,
                x: a.x, y: a.y, value_display: a.score,
                distance_display: a.distance_raw as f64 / 10.0,
                series_total: a.series_total, series_teiler_total: a.series_teiler_total,
            });
        });
        wait_ready.recv_timeout(Duration::from_secs(2)).unwrap();
        release.send(()).unwrap();
        assert!(!transition.join().unwrap().probe_active);
        ingest.join().unwrap();
        let state = engine.snapshot();
        assert!(!state.probe_active);
        assert_eq!(state.shots.len(), 1);
        assert_eq!(state.series_total, 10.0);
    }
}
