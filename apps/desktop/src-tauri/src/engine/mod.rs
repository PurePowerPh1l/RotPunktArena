//! Live stand engine — worker lifecycle + in-memory snapshot.
//! Persistence stays in `db`; UI only consumes snapshots/events.

mod poll;
mod series;
mod session_lifecycle;

use crate::db::{Database, SessionInfo, TrainingSaveInfo};
use crate::transport::simulator::SimulatorControl;
use crate::transport::{ConnectionStatus, TransportKind};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiShot {
    pub shot_index: u32,
    pub value_raw: i32,
    pub distance_raw: i32,
    pub x: i32,
    pub y: i32,
    pub value_display: f64,
    pub distance_display: f64,
    pub series_total: f64,
    /// Running Σ Teiler (distance_display) — server-side, not recomputed in UI.
    pub series_teiler_total: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionUpdate {
    pub status: ConnectionStatus,
    pub transport: TransportKind,
    pub port: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveState {
    /// Monotonic projection revision, allocated under the snapshot state lock.
    pub revision: u64,
    pub session_id: Option<String>,
    pub phase: String,
    pub status: ConnectionStatus,
    pub transport: TransportKind,
    pub port: Option<String>,
    pub session: Option<SessionInfo>,
    pub shots: Vec<UiShot>,
    pub series_total: f64,
    pub series_teiler_total: f64,
    pub last_shot: Option<UiShot>,
    pub auto_fire: bool,
    /// Legacy name: true when Cargo feature `rfcomm` is enabled (native hardware link).
    /// Not Virtual-COM / feature `serial`. FE field: `serialFeature`.
    pub serial_feature: bool,
    /// Shot limit; `None` = unlimited (e.g. training endless mode).
    pub max_shots: Option<i64>,
    /// True after max_shots reached and session auto-closed.
    pub series_complete: bool,
    /// Outcome of the last training history save decision (stop/reset).
    pub training_save: Option<TrainingSaveInfo>,
    /// Training endless mode — no series limit, never written to history/stats.
    pub endless_mode: bool,
    /// Probe phase active — shots are Probeschüsse (unscored, no limit).
    pub probe_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesCompletePayload {
    pub max_shots: i64,
    pub shot_count: i64,
    pub series_total: f64,
    pub shooter_name: String,
}

struct SharedInner {
    status: ConnectionStatus,
    transport: TransportKind,
    port: Option<String>,
    session: Option<SessionInfo>,
    shots: Vec<UiShot>,
    series_total: f64,
    series_teiler_total: f64,
    auto_fire: bool,
    max_shots: Option<i64>,
    series_complete: bool,
    last_training_save: Option<TrainingSaveInfo>,
    /// Preference + active session flag for training endless mode.
    endless_mode: bool,
    /// Preferred training series length (5/10/20/30) when not endless.
    training_series_shots: i64,
    /// Current session is in the probe phase (Probeschüsse before scoring).
    probe_active: bool,
}

pub struct StandEngine {
    projection_revision: AtomicU64,
    /// Lock order: ingest_gate -> log -> inner. Covers commit through projection.
    ingest_gate: Mutex<()>,
    lifecycle_gate: Mutex<()>,
    log: Mutex<Database>,
    inner: Mutex<SharedInner>,
    sim_control: SimulatorControl,
    stop: Arc<AtomicBool>,
    /// Bumped on each start/stop so stale workers stop applying state.
    generation: AtomicU64,
    worker: Mutex<Option<JoinHandle<()>>>,
    retired_workers: Mutex<Vec<JoinHandle<()>>>,
}

pub struct StartSessionArgs {
    pub shooter_name: String,
    pub use_simulator: bool,
    pub competition_id: Option<String>,
    pub entry_id: Option<String>,
    pub person_id: Option<String>,
    /// Only applies to training (`competition_id` is None).
    pub endless: bool,
}

impl StandEngine {
    pub fn new(log: Database) -> Self {
        Self {
            projection_revision: AtomicU64::new(0),
            ingest_gate: Mutex::new(()),
            lifecycle_gate: Mutex::new(()),
            log: Mutex::new(log),
            inner: Mutex::new(SharedInner {
                status: ConnectionStatus::Disconnected,
                transport: TransportKind::Simulator,
                port: None,
                session: None,
                shots: Vec::new(),
                series_total: 0.0,
                series_teiler_total: 0.0,
                auto_fire: false,
                max_shots: None,
                series_complete: false,
                last_training_save: None,
                endless_mode: false,
                training_series_shots: crate::db::TRAINING_SERIES_SHOTS,
                probe_active: false,
            }),
            sim_control: SimulatorControl::default(),
            stop: Arc::new(AtomicBool::new(false)),
            generation: AtomicU64::new(0),
            worker: Mutex::new(None),
            retired_workers: Mutex::new(Vec::new()),
        }
    }

    pub fn with_db<R>(&self, f: impl FnOnce(&Database) -> R) -> R {
        f(&self.log.lock())
    }

    pub fn with_db_mut<R>(&self, f: impl FnOnce(&mut Database) -> R) -> R {
        f(&mut self.log.lock())
    }

    /// Replace contents atomically while SQLite retains the live file/connection.
    pub fn swap_database_file(&self, source: &std::path::Path) -> Result<(), String> {
        self.replace_database(Some(source))
    }

    pub fn reset_database_to_empty(&self) -> Result<(), String> {
        self.replace_database(None)
    }

    fn replace_database(&self, source: Option<&std::path::Path>) -> Result<(), String> {
        let _lifecycle = self.lifecycle_gate.lock();
        if self.is_running() {
            return Err("Bitte zuerst die laufende Session in der Arena beenden".into());
        }
        self.stop_worker();
        let workers = std::mem::take(&mut *self.retired_workers.lock());
        let mut worker_failed = false;
        for worker in workers {
            if worker.join().is_err() {
                eprintln!("Poll-Worker ist während Wartungsbarriere fehlgeschlagen");
                worker_failed = true;
            }
        }
        if worker_failed {
            return Err("Worker-Abschluss fehlgeschlagen; Live-DB bleibt erhalten".into());
        }
        let _ingest = self.ingest_gate.lock();
        let mut live = self.log.lock();
        let _snapshots = crate::db::pause_snapshots();
        if live.path().as_os_str() == ":memory:" {
            return Err("Restore/Reset benötigt eine dateibasierte Live-DB".into());
        }
        let parent = live.path().parent().ok_or("Live-DB ohne Verzeichnis")?;
        let id = uuid::Uuid::new_v4();
        let staging = parent.join(format!(".replacement-{id}.sqlite"));
        let rollback = parent.join(format!("restore-rollback-{id}.sqlite"));
        let result = Database::prepare_replacement(source, &staging)
            .and_then(|candidate| live.replace_contents(&candidate, &rollback));
        // Candidate connection has closed; these are uniquely owned staging files.
        for path in [
            staging.clone(),
            std::path::PathBuf::from(format!("{}-wal", staging.display())),
            std::path::PathBuf::from(format!("{}-shm", staging.display())),
        ] {
            if let Err(error) = std::fs::remove_file(&path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!(
                        "Staging-Datei konnte nicht entfernt werden ({}): {error}",
                        path.display()
                    );
                }
            }
        }
        result?;
        // Keep validated rollback file; clear UI only after committed replacement.
        let mut state = self.inner.lock();
        state.session = None;
        state.shots.clear();
        state.series_total = 0.0;
        state.series_teiler_total = 0.0;
        state.series_complete = false;
        state.last_training_save = None;
        state.max_shots = None;
        state.auto_fire = false;
        state.status = ConnectionStatus::Disconnected;
        state.port = None;
        state.probe_active = false;
        Ok(())
    }
    pub fn snapshot(&self) -> LiveState {
        let g = self.inner.lock();
        LiveState {
            revision: self.projection_revision.fetch_add(1, Ordering::SeqCst) + 1,
            session_id: g.session.as_ref().map(|session| session.id.clone()),
            phase: if g.session.is_none() {
                "idle"
            } else if g
                .session
                .as_ref()
                .is_some_and(|session| session.ended_at.is_some())
            {
                "closed"
            } else if g.probe_active {
                "probe"
            } else {
                "match"
            }
            .into(),
            status: g.status,
            transport: g.transport,
            port: g.port.clone(),
            session: g.session.clone(),
            shots: g.shots.clone(),
            series_total: g.series_total,
            series_teiler_total: g.series_teiler_total,
            last_shot: g.shots.last().cloned(),
            auto_fire: g.auto_fire,
            // Legacy field name `serial_feature`: means RFCOMM/hardware link compiled in.
            serial_feature: cfg!(feature = "rfcomm"),
            max_shots: g.max_shots,
            series_complete: g.series_complete,
            training_save: g.last_training_save.clone(),
            endless_mode: g.endless_mode,
            probe_active: g.probe_active,
        }
    }

    pub(crate) fn emit_live<R: tauri::Runtime>(&self, app: &AppHandle<R>, detail: Option<String>) {
        let state = self.snapshot();
        if let Err(error) = app.emit(
            "live_state",
            serde_json::json!({ "state": state, "detail": detail }),
        ) {
            eprintln!("Live-Zustand konnte nicht angezeigt werden: {error}");
        }
    }

    #[cfg(test)]
    pub fn apply_connection_update(&self, u: &ConnectionUpdate) {
        let mut g = self.inner.lock();
        g.status = u.status;
        g.transport = u.transport;
        g.port = u.port.clone();
    }

    /// Idempotent: ignore duplicate shotIndex from stale emits / double apply.
    pub fn apply_shot(&self, shot: UiShot) -> bool {
        let mut g = self.inner.lock();
        if g.shots.iter().any(|s| s.shot_index == shot.shot_index) {
            return false;
        }
        g.series_total = shot.series_total;
        g.series_teiler_total = shot.series_teiler_total;
        g.shots.push(shot);
        g.status = ConnectionStatus::Connected;
        true
    }

    /// Poll pre-gate: bound `session_id` is the current open, non-complete series.
    pub(crate) fn poll_session_accepting(&self, session_id: &str) -> bool {
        let g = self.inner.lock();
        if g.series_complete {
            return false;
        }
        g.session
            .as_ref()
            .is_some_and(|s| s.id == session_id && s.ended_at.is_none())
    }

    /// DIAGNOSE-ONLY: "training" | "competition" for latency JSONL.
    pub(crate) fn session_mode_label(&self) -> Option<&'static str> {
        let g = self.inner.lock();
        g.session.as_ref().map(|s| {
            if s.competition_id.is_some() {
                "competition"
            } else {
                "training"
            }
        })
    }
}

pub(crate) fn emit_conn(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    u: ConnectionUpdate,
) {
    {
        let mut state = engine.inner.lock();
        if engine.generation.load(Ordering::SeqCst) != generation {
            return;
        }
        state.status = u.status;
        state.transport = u.transport;
        state.port = u.port.clone();
    }
    engine.emit_live(app, u.detail.clone());
    let _ = app.emit("connection", u);
}

#[cfg(test)]
mod maintenance_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn live_projection_contract_and_revision_order() {
        let engine = StandEngine::new(Database::open_in_memory().unwrap());
        let mut snapshot = engine.snapshot();
        snapshot.serial_feature = false; // fixture is independent of compiled adapters
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../../../fixtures/live-idle.json")).unwrap();
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), fixture);
        assert_eq!(
            serde_json::from_value::<LiveState>(fixture).unwrap().phase,
            "idle"
        );
        assert!(engine.snapshot().revision > snapshot.revision);
        engine.apply_connection_update(&ConnectionUpdate {
            status: ConnectionStatus::Searching,
            transport: TransportKind::Rfcomm,
            port: None,
            detail: None,
        });
        let hardware = engine.snapshot();
        assert!(hardware.revision > snapshot.revision);
        assert_eq!(
            serde_json::to_value(hardware).unwrap()["transport"],
            "rfcomm"
        );
    }

    #[test]
    fn restore_waits_for_delayed_retired_worker_before_replacement() {
        let dir = std::env::temp_dir().join(format!("reddot-maintenance-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.sqlite");
        let backup = Database::open(&source).unwrap();
        backup.set_setting("barrier_test", "replacement").unwrap();
        drop(backup);
        let engine = Arc::new(StandEngine::new(
            Database::open(dir.join("live.sqlite")).unwrap(),
        ));
        let (entered, running) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let old_engine = engine.clone();
        let worker = std::thread::spawn(move || {
            entered.send(()).unwrap();
            wait.recv().unwrap();
            old_engine
                .with_db(|db| db.set_setting("barrier_test", "old-worker"))
                .unwrap();
        });
        running.recv_timeout(Duration::from_secs(2)).unwrap();
        engine.retired_workers.lock().push(worker);
        let (done, result) = mpsc::channel();
        let restoring = engine.clone();
        let restore =
            std::thread::spawn(move || done.send(restoring.swap_database_file(&source)).unwrap());
        assert!(result.recv_timeout(Duration::from_millis(100)).is_err());
        release.send(()).unwrap();
        result
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        restore.join().unwrap();
        assert_eq!(
            engine
                .with_db(|db| db.get_setting("barrier_test"))
                .unwrap()
                .as_deref(),
            Some("replacement")
        );
        assert!(engine.retired_workers.lock().is_empty());
        drop(engine);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
