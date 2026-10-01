//! Restore via SQLite's atomic backup transaction; never remove the live file.
use super::Database;
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use std::path::Path;

fn check_integrity(conn: &Connection) -> Result<(), String> {
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err(format!("DB-Integrität: {integrity}"));
    }
    let mut check = conn
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| e.to_string())?;
    if check
        .query([])
        .map_err(|e| e.to_string())?
        .next()
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("DB enthält ungültige Fremdschlüssel".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (std::path::PathBuf, Database, Database) {
        let dir = std::env::temp_dir().join(format!("rotpunkt-restore-{}", uuid::Uuid::new_v4()));
        let live = Database::open(dir.join("live.sqlite")).unwrap();
        let candidate = Database::open(dir.join("candidate.sqlite")).unwrap();
        live.set_setting("fixture", "original").unwrap();
        candidate.set_setting("fixture", "replacement").unwrap();
        (dir, live, candidate)
    }

    #[test]
    fn invalid_backup_and_unsupported_schema_leave_original_usable() {
        let (dir, live, candidate) = fixture();
        let invalid = dir.join("invalid.sqlite");
        std::fs::write(&invalid, b"invalid backup").unwrap();
        assert!(Database::prepare_replacement(Some(&invalid), &dir.join("stage.sqlite")).is_err());
        candidate
            .conn
            .execute(
                "INSERT INTO schema_migrations VALUES (999, 'future', 'fixture')",
                [],
            )
            .unwrap();
        assert!(
            Database::prepare_replacement(Some(candidate.path()), &dir.join("stage.sqlite"))
                .is_err()
        );
        assert!(Database::prepare_replacement(
            Some(&dir.join("missing.sqlite")),
            &dir.join("stage.sqlite")
        )
        .is_err());
        assert_eq!(
            live.get_setting("fixture").unwrap().as_deref(),
            Some("original")
        );
        live.set_setting("after_error", "usable").unwrap();
        drop(candidate);
        drop(live);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn interrupted_copy_rolls_back_without_removing_live_database() {
        let (dir, mut live, candidate) = fixture();
        {
            let copy = Backup::new(&candidate.conn, &mut live.conn).unwrap();
            assert!(matches!(copy.step(1).unwrap(), StepResult::More));
            // Drop before Done is a rollback, including already copied pages.
        }
        assert_eq!(
            live.get_setting("fixture").unwrap().as_deref(),
            Some("original")
        );
        assert!(live.path().is_file());
        drop(candidate);
        drop(live);
        let reopened = Database::open(dir.join("live.sqlite")).unwrap();
        assert_eq!(
            reopened.get_setting("fixture").unwrap().as_deref(),
            Some("original")
        );
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_post_open_validation_rolls_back_then_retry_and_reset_succeed() {
        let (dir, mut live, candidate) = fixture();
        assert!(live
            .replace_contents_checked(&candidate, &dir.join("rollback.sqlite"), |_| Err(
                "injected validation".into()
            ))
            .is_err());
        assert_eq!(
            live.get_setting("fixture").unwrap().as_deref(),
            Some("original")
        );
        live.replace_contents(&candidate, &dir.join("retry-rollback.sqlite"))
            .unwrap();
        assert_eq!(
            live.get_setting("fixture").unwrap().as_deref(),
            Some("replacement")
        );
        let rollback = Database::open(dir.join("retry-rollback.sqlite")).unwrap();
        assert_eq!(
            rollback.get_setting("fixture").unwrap().as_deref(),
            Some("original")
        );
        let empty = Database::prepare_replacement(None, &dir.join("empty.sqlite")).unwrap();
        live.replace_contents(&empty, &dir.join("reset-rollback.sqlite"))
            .unwrap();
        assert!(live.get_setting("fixture").unwrap().is_none());
        assert_eq!(live.path(), dir.join("live.sqlite"));
        drop(empty);
        drop(rollback);
        drop(candidate);
        drop(live);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_rollback_file_creation_cannot_change_live_contents() {
        let (dir, mut live, candidate) = fixture();
        let blocked = dir.join("blocked.sqlite");
        std::fs::create_dir(&blocked).unwrap();
        assert!(live.replace_contents(&candidate, &blocked).is_err());
        assert_eq!(
            live.get_setting("fixture").unwrap().as_deref(),
            Some("original")
        );
        drop(candidate);
        drop(live);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

fn copy_database(source: &Connection, destination: &mut Connection) -> Result<(), String> {
    let copy =
        Backup::new(source, destination).map_err(|e| format!("DB-Kopie vorbereiten: {e}"))?;
    // One step owns the destination transaction. Incomplete/error copies roll
    // back on Backup drop. Busy/Locked are explicit failures, never endless retries.
    match copy.step(-1).map_err(|e| format!("DB-Kopie: {e}"))? {
        StepResult::Done => Ok(()),
        other => Err(format!("DB-Kopie nicht abgeschlossen: {other:?}")),
    }
}

impl Database {
    pub(crate) fn prepare_replacement(
        source: Option<&Path>,
        staging: &Path,
    ) -> Result<Self, String> {
        if let Some(source) = source {
            let input = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("Backup öffnen: {e}"))?;
            check_integrity(&input)?;
            super::migrate::validate_backup_schema(&input)?;
            let mut output = Connection::open(staging).map_err(|e| e.to_string())?;
            copy_database(&input, &mut output)?;
        }
        let candidate = Self::open(staging)?;
        candidate.validate_replacement()?;
        Ok(candidate)
    }

    fn validate_replacement(&self) -> Result<(), String> {
        check_integrity(&self.conn)?;
        super::migrate::validate_backup_schema(&self.conn)?;
        // Exercise the columns used by core persistence before touching live data.
        for sql in [
            "SELECT id, phase, max_shots, next_sequence FROM sessions LIMIT 0",
            "SELECT frame_id, classification, score FROM shots LIMIT 0",
            "SELECT session_id, sequence, payload FROM events LIMIT 0",
            "SELECT raw_frame_hex, frame_sha256 FROM frames LIMIT 0",
            "SELECT key, value FROM settings LIMIT 0",
        ] {
            self.conn
                .prepare(sql)
                .map_err(|e| format!("Backup-Schema: {e}"))?;
        }
        Ok(())
    }

    pub(crate) fn replace_contents(
        &mut self,
        candidate: &Database,
        rollback_path: &Path,
    ) -> Result<(), String> {
        self.replace_contents_checked(candidate, rollback_path, Self::validate_replacement)
    }

    fn replace_contents_checked(
        &mut self,
        candidate: &Database,
        rollback_path: &Path,
        check: impl FnOnce(&Self) -> Result<(), String>,
    ) -> Result<(), String> {
        candidate.validate_replacement()?;
        self.vacuum_into(rollback_path)?;
        let rollback = Connection::open_with_flags(rollback_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Rückfallstand öffnen: {e}"))?;
        check_integrity(&rollback)?;
        copy_database(&candidate.conn, &mut self.conn)?;
        if let Err(error) = check(self) {
            return match copy_database(&rollback, &mut self.conn) {
                Ok(()) => Err(format!(
                    "Ersatzprüfung fehlgeschlagen; Rückfallstand wiederhergestellt: {error}"
                )),
                Err(restore_error) => Err(format!(
                    "Ersatzprüfung: {error}; Rollback: {restore_error}; Rückfallstand: {}",
                    rollback_path.display()
                )),
            };
        }
        Ok(())
    }
}
