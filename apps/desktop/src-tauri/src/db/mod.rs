//! SQLite source of truth — WAL, versioned migrations, single-writer Arena Core.

mod competitions;
mod domain_constants;
mod migrate;
mod people;
mod recovery;
mod replacement;
mod results;
mod session_start;
mod sessions;
mod snapshot_worker;
mod snapshots;
mod teams;
mod training;

use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub use competitions::{
    count_scored_shots_for_limit, session_effective_max_shots, session_tenths_enabled, Competition,
    CompetitionEntry, CreateCompetition,
};
pub use domain_constants::{
    competition_kind, competition_status, entry_status, event_kind, session_phase,
    shot_classification,
};
pub use people::{CreatePerson, Person, PromoteTrainingShooterResult};
pub use recovery::{RecoverySessionInfo, StoredUiShot};
pub use results::{EntryResultDetail, EntryResultSummary, SeriesResultSummary};
pub use sessions::{append_event_in_tx, session_phase_in_tx, touch_autosave_in_tx, SessionInfo};
pub(crate) use snapshot_worker::{pause_snapshots, shutdown_snapshots};
pub use snapshots::{SNAPSHOT_EVERY_N_SHOTS, SNAPSHOT_SUBDIR};
pub use teams::{CompetitionTeam, TeamResultSummary};
pub use training::{
    normalize_training_series_shots, TrainingSaveInfo, TrainingSessionDetail,
    TrainingSessionSummary, TrainingShooterOption, TRAINING_HISTORY_MIN_SHOTS,
    TRAINING_SERIES_SHOTS,
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
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )
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
        let mut rows = stmt
            .query(rusqlite::params![key])
            .map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod durability_tests {
    use super::*;

    #[test]
    fn every_file_connection_uses_full_durability() {
        let directory =
            std::env::temp_dir().join(format!("rotpunkt-durability-{}", uuid::Uuid::new_v4()));
        let path = directory.join("fixture.sqlite");
        for _ in 0..2 {
            let db = Database::open(&path).unwrap();
            assert_eq!(
                db.conn
                    .query_row("PRAGMA synchronous", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                2
            );
            assert_eq!(
                db.conn
                    .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "wal"
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn measure_committed_shot_latency_normal_vs_full() {
        let directory =
            std::env::temp_dir().join(format!("rotpunkt-latency-{}", uuid::Uuid::new_v4()));
        for mode in ["NORMAL", "FULL"] {
            let path = directory.join(format!("{mode}.sqlite"));
            let mut db = Database::open(&path).unwrap();
            db.conn
                .execute_batch(&format!("PRAGMA synchronous={mode}"))
                .unwrap();
            // No boundary jobs: this isolates the real Arena commit path.
            db.conn
                .execute(
                    "INSERT INTO sessions (id, shooter_name, started_at, next_sequence, phase)
                VALUES ('latency-fixture', 'Fixture', '2026-10-01', 1, 'match')",
                    [],
                )
                .unwrap();
            let mut latencies = Vec::new();
            for _ in 0..200 {
                let mut frame =
                    crate::protocol::build_synthetic_shot_frame("10.0", "001.00", "00001", "00002")
                        .unwrap();
                crate::protocol::stamp_frame_nonce(&mut frame);
                let start = std::time::Instant::now();
                assert!(matches!(
                    db.ingest_raw_frame("latency-fixture", &frame, "test", None)
                        .unwrap(),
                    crate::arena::IngestOutcome::Accepted(_)
                ));
                latencies.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            latencies.sort_by(f64::total_cmp);
            println!(
                "shot commit {mode}: n=200 median={:.3}ms p95={:.3}ms max={:.3}ms",
                latencies[100], latencies[190], latencies[199]
            );
            drop(db);
            let reopened = Database::open(&path).unwrap();
            assert_eq!(
                reopened.count_session_shots("latency-fixture").unwrap(),
                200
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
