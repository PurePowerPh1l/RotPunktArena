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
    log: Mutex<Database>,
    inner: Mutex<SharedInner>,
    sim_control: SimulatorControl,
    stop: Arc<AtomicBool>,
    /// Bumped on each start/stop so stale workers stop applying state.
    generation: AtomicU64,
    worker: Mutex<Option<JoinHandle<()>>>,
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
        }
    }

    pub fn with_db<R>(&self, f: impl FnOnce(&Database) -> R) -> R {
        f(&self.log.lock())
    }

    pub fn with_db_mut<R>(&self, f: impl FnOnce(&mut Database) -> R) -> R {
        f(&mut self.log.lock())
    }

    /// Stop any live worker and clear in-memory session UI state.
    fn clear_live_ui_state(&self) {
        self.stop_worker();
        let mut g = self.inner.lock();
        g.session = None;
        g.shots.clear();
        g.series_total = 0.0;
        g.series_teiler_total = 0.0;
        g.series_complete = false;
        g.last_training_save = None;
        g.max_shots = None;
        g.auto_fire = false;
        g.status = ConnectionStatus::Disconnected;
        g.port = None;
        g.probe_active = false;
    }

    /// Replace the on-disk DB with `source` (backup). Live session must be idle.
    pub fn swap_database_file(&self, source: &std::path::Path) -> Result<(), String> {
        if self.is_running() {
            return Err("Bitte zuerst die laufende Session in der Arena beenden".into());
        }
        if !source.is_file() {
            return Err("Backup-Datei nicht gefunden".into());
        }
        self.clear_live_ui_state();

        let live_path = {
            let mut guard = self.log.lock();
            let path = guard.path().to_path_buf();
            if path.to_string_lossy() == ":memory:" {
                return Err("In-Memory-DB kann nicht aus Datei wiederhergestellt werden".into());
            }
            // Release file locks before replacing the file.
            *guard = Database::open_in_memory().map_err(|e| e.to_string())?;
            path
        };

        let (wal, shm) = (
            std::path::PathBuf::from(format!("{}-wal", live_path.display())),
            std::path::PathBuf::from(format!("{}-shm", live_path.display())),
        );
        let _ = std::fs::remove_file(&wal);
        let _ = std::fs::remove_file(&shm);
        if live_path.exists() {
            std::fs::remove_file(&live_path).map_err(|e| format!("Alte DB entfernen: {e}"))?;
        }
        std::fs::copy(source, &live_path).map_err(|e| format!("Backup kopieren: {e}"))?;

        let new_db = Database::open(&live_path).map_err(|e| e.to_string())?;
        *self.log.lock() = new_db;
        Ok(())
    }

    /// Replace live DB with a freshly migrated empty database (same path).
    pub fn reset_database_to_empty(&self) -> Result<(), String> {
        if self.is_running() {
            return Err("Bitte zuerst die laufende Session in der Arena beenden".into());
        }
        self.clear_live_ui_state();

        let live_path = {
            let mut guard = self.log.lock();
            let path = guard.path().to_path_buf();
            if path.to_string_lossy() == ":memory:" {
                *guard = Database::open_in_memory().map_err(|e| e.to_string())?;
                return Ok(());
            }
            *guard = Database::open_in_memory().map_err(|e| e.to_string())?;
            path
        };

        let (wal, shm) = (
            std::path::PathBuf::from(format!("{}-wal", live_path.display())),
            std::path::PathBuf::from(format!("{}-shm", live_path.display())),
        );
        let _ = std::fs::remove_file(&wal);
        let _ = std::fs::remove_file(&shm);
        if live_path.exists() {
            std::fs::remove_file(&live_path).map_err(|e| format!("Alte DB entfernen: {e}"))?;
        }
        let new_db = Database::open(&live_path).map_err(|e| e.to_string())?;
        *self.log.lock() = new_db;
        Ok(())
    }

    pub fn snapshot(&self) -> LiveState {
        let g = self.inner.lock();
        LiveState {
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
        g.session.as_ref().is_some_and(|s| {
            s.id == session_id && s.ended_at.is_none()
        })
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
    if engine.generation.load(Ordering::SeqCst) != generation {
        return;
    }
    engine.apply_connection_update(&u);
    let _ = app.emit("connection", u);
}
