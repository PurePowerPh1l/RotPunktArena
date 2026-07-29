//! SQLite source of truth — WAL, versioned migrations, single-writer Arena Core.

mod competitions;
mod domain_constants;
mod migrate;
mod people;
mod recovery;
mod results;
mod sessions;
mod snapshots;
mod teams;
mod training;

use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub use competitions::{
    count_scored_shots_for_limit, session_effective_max_shots, session_tenths_enabled,
    Competition, CompetitionEntry, CreateCompetition,
};
pub use domain_constants::{competition_kind, competition_status, entry_status, event_kind};
pub use people::{CreatePerson, Person, PromoteTrainingShooterResult};
pub use recovery::{RecoverySessionInfo, StoredUiShot};
pub use results::{EntryResultDetail, EntryResultSummary, SeriesResultSummary};
pub use sessions::{append_event_in_tx, touch_autosave_in_tx, SessionInfo};
pub use snapshots::{SNAPSHOT_EVERY_N_SHOTS, SNAPSHOT_SUBDIR};
pub use teams::{CompetitionTeam, TeamResultSummary};
pub use training::{
    TrainingSaveInfo, TrainingSessionDetail, TrainingSessionSummary, TrainingShooterOption,
    TRAINING_HISTORY_MIN_SHOTS, TRAINING_SERIES_SHOTS,
};

pub struct Database {
    pub(crate) conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        Self::configure(&conn)?;
        let db = Self { conn, path };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::configure(&conn)?;
        let db = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.migrate()?;
        Ok(db)
    }

    fn configure(conn: &Connection) -> Result<(), String> {
        conn.busy_timeout(std::time::Duration::from_millis(3000))
            .map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<(), String> {
        migrate::apply_migrations(&self.conn)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(rusqlite::params![key]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(row.get(0).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
