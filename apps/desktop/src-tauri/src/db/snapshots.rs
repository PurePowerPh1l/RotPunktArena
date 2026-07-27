//! WAL-safe DB file snapshots via `VACUUM INTO` (never raw file copy of the live DB).
//!
//! Triggers (call sites): session start/end, every [`SNAPSHOT_EVERY_N_SHOTS`] accepted shots.
//! Failures are best-effort — callers should use `try_*` helpers so ingest/lifecycle stay up.

use super::Database;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Accepted-shot cadence for hybrid snapshots (event-count, not wall-clock).
pub const SNAPSHOT_EVERY_N_SHOTS: i64 = 100;
/// Keep the newest N snapshot files per session id.
pub const SNAPSHOT_RETAIN_PER_SESSION: usize = 5;
pub const SNAPSHOT_SUBDIR: &str = "snapshots";
const SNAPSHOT_LATEST_NAME: &str = "latest.sqlite";

impl Database {
    /// `…/snapshots` next to the live DB. `None` for in-memory / invalid paths.
    pub fn snapshot_dir(&self) -> Option<PathBuf> {
        let parent = self.path.parent()?;
        if self.path.as_os_str() == ":memory:" || parent.as_os_str().is_empty() {
            return None;
        }
        Some(parent.join(SNAPSHOT_SUBDIR))
    }

    /// Best-effort snapshot at session boundary. Logs on failure; never returns Err.
    pub fn try_session_boundary_snapshot(&self, session_id: &str) {
        let seq = self.last_event_sequence(session_id).unwrap_or(0);
        if let Err(e) = self.write_session_snapshot(session_id, seq) {
            eprintln!("[reddot] session snapshot failed ({session_id}): {e}");
        }
    }

    /// After an accepted shot (outside ingest TX): snapshot when `shot_index % N == 0`.
    pub fn try_maybe_snapshot_after_shot(
        &self,
        session_id: &str,
        shot_index: i32,
        session_sequence: i64,
    ) {
        if i64::from(shot_index) % SNAPSHOT_EVERY_N_SHOTS != 0 {
            return;
        }
        if let Err(e) = self.write_session_snapshot(session_id, session_sequence) {
            eprintln!(
                "[reddot] shot-cadence snapshot failed ({session_id} @ {shot_index}): {e}"
            );
        }
    }

    /// Consistent snapshot via [`Database::vacuum_into`], then retention + `latest.sqlite`.
    pub fn write_session_snapshot(
        &self,
        session_id: &str,
        sequence: i64,
    ) -> Result<PathBuf, String> {
        let dir = self
            .snapshot_dir()
            .ok_or_else(|| "Snapshots nur für dateibasierte DBs".to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let safe_id = sanitize_id(session_id);
        let file_name = format!("session-{safe_id}-seq-{sequence}-{ms}.sqlite");
        let dest = dir.join(&file_name);

        self.vacuum_into(&dest)?;

        // `latest` is a copy of an already-consistent VACUUM INTO artifact (not the live WAL DB).
        let latest = dir.join(SNAPSHOT_LATEST_NAME);
        if latest.exists() {
            let _ = std::fs::remove_file(&latest);
        }
        std::fs::copy(&dest, &latest).map_err(|e| format!("latest snapshot: {e}"))?;

        retain_session_snapshots(&dir, &safe_id, SNAPSHOT_RETAIN_PER_SESSION)?;
        Ok(dest)
    }

    fn last_event_sequence(&self, session_id: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) FROM events WHERE session_id = ?1",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }
}

fn sanitize_id(session_id: &str) -> String {
    session_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect()
}

fn retain_session_snapshots(dir: &Path, safe_id: &str, keep: usize) -> Result<(), String> {
    let prefix = format!("session-{safe_id}-");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".sqlite"))
        })
        .collect();

    files.sort_by(|a, b| {
        let ma = a
            .metadata()
            .and_then(|m| m.modified())
            .ok();
        let mb = b
            .metadata()
            .and_then(|m| m.modified())
            .ok();
        ma.cmp(&mb)
    });

    let excess = files.len().saturating_sub(keep);
    for path in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db() -> (PathBuf, Database) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("reddot-snap-unit-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.sqlite");
        let db = Database::open(&path).unwrap();
        (dir, db)
    }

    #[test]
    fn memory_db_skips_snapshot_dir() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.snapshot_dir().is_none());
    }

    #[test]
    fn write_session_snapshot_creates_file_and_latest() {
        let (dir, mut db) = temp_db();
        let session = db.start_session("Snap", None, None, None).unwrap();
        // start_session already tried a boundary snapshot
        let snap_dir = db.snapshot_dir().unwrap();
        assert!(snap_dir.is_dir());
        let entries: Vec<_> = std::fs::read_dir(&snap_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            entries.len() >= 2,
            "expected session-*.sqlite + latest.sqlite, got {entries:?}"
        );
        assert!(snap_dir.join(SNAPSHOT_LATEST_NAME).is_file());

        let path = db.write_session_snapshot(&session.id, 42).unwrap();
        assert!(path.is_file());
        assert!(path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("seq-42"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
