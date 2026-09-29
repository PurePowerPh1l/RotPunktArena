//! WAL-safe DB file snapshots via `VACUUM INTO` (never raw file copy of the live DB).
//!
//! Triggers (call sites): session start/end, every [`SNAPSHOT_EVERY_N_SHOTS`] accepted shots.
//! Failures are best-effort — callers should use `try_*` helpers so ingest/lifecycle stay up.

use super::Database;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Single-flight guard for background cadence snapshots.
static SNAPSHOT_WRITE_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

static CADENCE_SNAPSHOT_RUNNING: AtomicBool = AtomicBool::new(false);

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
    ///
    /// Synchronous — holds the caller (and the engine DB mutex) for the whole
    /// `VACUUM INTO`. Prefer [`Database::spawn_session_boundary_snapshot`] on
    /// lifecycle paths so session start/end never wait on VACUUM I/O.
    pub fn try_session_boundary_snapshot(&self, session_id: &str) {
        let seq = self.last_event_sequence(session_id).unwrap_or(0);
        if let Err(e) = self.write_session_snapshot(session_id, seq) {
            eprintln!("[reddot] session snapshot failed ({session_id}): {e}");
        }
    }

    /// Async session-boundary snapshot: runs `VACUUM INTO` on a background
    /// thread with a fresh connection (WAL-safe) so session start/end return
    /// immediately. Start/end requests are retained; the writer serializes publication.
    /// No-op for in-memory DBs.
    pub fn spawn_session_boundary_snapshot(&self, session_id: &str) {
        if self.snapshot_dir().is_none() {
            return;
        }
        let path = self.path().to_path_buf();
        let sid = session_id.to_string();
        std::thread::spawn(move || {
            let result = Database::open(&path).and_then(|db| {
                let seq = db.last_event_sequence(&sid).unwrap_or(0);
                db.write_session_snapshot(&sid, seq).map(|_| ())
            });
            if let Err(e) = result {
                eprintln!("[reddot] session boundary snapshot failed ({sid}): {e}");
            }
        });
    }

    /// After an accepted shot (outside ingest TX): snapshot when `shot_index % N == 0`.
    /// Synchronous — prefer [`Database::spawn_maybe_snapshot_after_shot`] on
    /// live paths so device ACK / UI emit never wait on VACUUM I/O.
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
            eprintln!("[reddot] shot-cadence snapshot failed ({session_id} @ {shot_index}): {e}");
        }
    }

    /// Async cadence snapshot for live ingest paths: runs `VACUUM INTO` on a
    /// background thread with a fresh connection (WAL-safe). Single-flight —
    /// a still-running snapshot skips the new request (best-effort cadence).
    pub fn spawn_maybe_snapshot_after_shot(
        &self,
        session_id: &str,
        shot_index: i32,
        session_sequence: i64,
    ) {
        if i64::from(shot_index) % SNAPSHOT_EVERY_N_SHOTS != 0 {
            return;
        }
        if self.snapshot_dir().is_none() {
            return;
        }
        if CADENCE_SNAPSHOT_RUNNING.swap(true, Ordering::SeqCst) {
            return;
        }
        let path = self.path().to_path_buf();
        let sid = session_id.to_string();
        std::thread::spawn(move || {
            let result = Database::open(&path).and_then(|db| {
                db.write_session_snapshot(&sid, session_sequence)
                    .map(|_| ())
            });
            CADENCE_SNAPSHOT_RUNNING.store(false, Ordering::SeqCst);
            if let Err(e) = result {
                eprintln!("[reddot] shot-cadence snapshot failed ({sid} @ {shot_index}): {e}");
            }
        });
    }

    /// Consistent snapshot via [`Database::vacuum_into`], then retention + `latest.sqlite`.
    pub fn write_session_snapshot(
        &self,
        session_id: &str,
        sequence: i64,
    ) -> Result<PathBuf, String> {
        let _guard = SNAPSHOT_WRITE_LOCK.lock();
        let dir = self
            .snapshot_dir()
            .ok_or_else(|| "Snapshots nur für dateibasierte DBs".to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let safe_id = sanitize_id(session_id);
        let file_name = format!(
            "session-{safe_id}-seq-{sequence}-{ms}-{}.sqlite",
            uuid::Uuid::new_v4()
        );
        let dest = dir.join(&file_name);

        let temp = dest.with_extension("tmp");
        let latest_temp = dir.join(".latest.tmp");
        let result = (|| -> Result<(), String> {
            self.vacuum_into(&temp)?;
            std::fs::copy(&temp, &latest_temp).map_err(|e| e.to_string())?;
            std::fs::OpenOptions::new()
                .write(true)
                .open(&latest_temp)
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string())?;
            std::fs::rename(&latest_temp, dir.join(SNAPSHOT_LATEST_NAME))
                .map_err(|e| format!("latest snapshot: {e}"))?;
            // Only completed snapshots get the public .sqlite name.
            std::fs::rename(&temp, &dest).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
            let _ = std::fs::remove_file(&latest_temp);
        }
        result?;

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
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
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
        let ma = a.metadata().and_then(|m| m.modified()).ok();
        let mb = b.metadata().and_then(|m| m.modified()).ok();
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
        // Boundary snapshot now runs on a background thread; exercise the
        // synchronous writer directly so the test is deterministic.
        let first = db.write_session_snapshot(&session.id, 1).unwrap();
        assert!(first.is_file());

        let snap_dir = db.snapshot_dir().unwrap();
        assert!(snap_dir.is_dir());
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
    #[cfg(windows)]
    #[test]
    fn failed_publication_keeps_previous_snapshot_and_retry_is_complete() {
        use std::os::windows::fs::OpenOptionsExt;
        let (dir, db) = temp_db();
        db.set_setting("snapshot_test", "before").unwrap();
        db.write_session_snapshot("test", 1).unwrap();
        let snapshots = db.snapshot_dir().unwrap();
        let latest = snapshots.join(SNAPSHOT_LATEST_NAME);
        let before = std::fs::read(&latest).unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&latest)
            .unwrap();
        db.set_setting("snapshot_test", "after").unwrap();
        assert!(db.write_session_snapshot("test", 2).is_err());
        assert_eq!(std::fs::read(&latest).unwrap(), before);
        assert_eq!(std::fs::read_dir(&snapshots).unwrap().count(), 2);
        drop(locked);
        db.write_session_snapshot("test", 2).unwrap();
        let snapshot = rusqlite::Connection::open_with_flags(
            &latest,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            snapshot
                .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            snapshot
                .query_row(
                    "SELECT value FROM settings WHERE key = 'snapshot_test'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "after"
        );
        drop(snapshot);
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
