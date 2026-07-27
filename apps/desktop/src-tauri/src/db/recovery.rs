//! Recovery queries — interrupted sessions via autosave marker, UI shot rehydration, event dump.

use super::sessions::recovery_state;
use super::{entry_status, sessions::SessionInfo, Database};
use rusqlite::params;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySessionInfo {
    pub id: String,
    pub shooter_name: String,
    pub started_at: String,
    pub competition_id: Option<String>,
    pub entry_id: Option<String>,
    pub person_id: Option<String>,
    pub shot_count: i64,
    /// Last committed event sequence at autosave (shot-level progress).
    pub last_autosave_sequence: Option<i64>,
    /// ISO timestamp of last autosave / heartbeat.
    pub last_autosave_at: Option<String>,
    pub recovery_state: String,
}

#[derive(Debug, Clone)]
pub struct StoredUiShot {
    pub shot_index: u32,
    pub value_raw: i32,
    pub distance_raw: i32,
    pub x: i32,
    pub y: i32,
    pub value_display: f64,
    pub distance_display: f64,
    pub series_total: f64,
    pub series_teiler_total: f64,
}

impl Database {
    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, shooter_name, started_at, ended_at,
                        competition_id, entry_id, person_id
                 FROM sessions WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![session_id])
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(SessionInfo {
                id: row.get(0).map_err(|e| e.to_string())?,
                shooter_name: row.get(1).map_err(|e| e.to_string())?,
                started_at: row.get(2).map_err(|e| e.to_string())?,
                ended_at: row.get(3).map_err(|e| e.to_string())?,
                competition_id: row.get(4).map_err(|e| e.to_string())?,
                entry_id: row.get(5).map_err(|e| e.to_string())?,
                person_id: row.get(6).map_err(|e| e.to_string())?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Boot check: open sessions with an autosave marker (interrupted after unclean shutdown).
    /// Does not mutate live `active` rows — the gate treats open+autosave as interrupted.
    pub fn list_recovery_sessions(&self) -> Result<Vec<RecoverySessionInfo>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, s.shooter_name, s.started_at,
                        s.competition_id, s.entry_id, s.person_id,
                        (SELECT COUNT(*) FROM shots sh WHERE sh.session_id = s.id),
                        s.last_autosave_sequence, s.last_autosave_at, s.recovery_state
                 FROM sessions s
                 WHERE s.ended_at IS NULL
                   AND s.last_autosave_at IS NOT NULL
                 ORDER BY s.last_autosave_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let raw_state: String = r.get(9)?;
                let effective = match raw_state.as_str() {
                    "recovered" | "interrupted" => raw_state,
                    _ => recovery_state::INTERRUPTED.to_string(),
                };
                Ok(RecoverySessionInfo {
                    id: r.get(0)?,
                    shooter_name: r.get(1)?,
                    started_at: r.get(2)?,
                    competition_id: r.get(3)?,
                    entry_id: r.get(4)?,
                    person_id: r.get(5)?,
                    shot_count: r.get(6)?,
                    last_autosave_sequence: r.get(7)?,
                    last_autosave_at: r.get(8)?,
                    recovery_state: effective,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// Open session ids (ended_at IS NULL) — used by diagnostics / export.
    pub fn list_unclean_sessions(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id FROM sessions
                 WHERE ended_at IS NULL AND last_autosave_at IS NOT NULL
                 ORDER BY COALESCE(last_autosave_at, started_at) DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// Load scored shots ascending for engine rehydration (running series_total).
    pub fn load_session_ui_shots(&self, session_id: &str) -> Result<Vec<StoredUiShot>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT shot_index, score, value_raw, distance_raw, x, y
                 FROM shots WHERE session_id = ?1
                 ORDER BY shot_index ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |r| {
                Ok((
                    r.get::<_, i32>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, i32>(2)?,
                    r.get::<_, i32>(3)?,
                    r.get::<_, i32>(4)?,
                    r.get::<_, i32>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut running = 0.0_f64;
        let mut running_teiler = 0.0_f64;
        for r in rows {
            let (shot_index, score, value_raw, distance_raw, x, y) =
                r.map_err(|e| e.to_string())?;
            let distance_display = distance_raw as f64 / 10.0;
            running += score;
            running_teiler += distance_display;
            out.push(StoredUiShot {
                shot_index: shot_index as u32,
                value_raw,
                distance_raw,
                x,
                y,
                value_display: score,
                distance_display,
                series_total: running,
                series_teiler_total: running_teiler,
            });
        }
        Ok(out)
    }

    /// Safely close an interrupted session (Recovery Gate: "Sicher abschließen").
    pub fn close_interrupted_session(&mut self, session_id: &str) -> Result<(), String> {
        let session = self
            .get_session(session_id)?
            .ok_or_else(|| "Session nicht gefunden".to_string())?;
        if session.ended_at.is_some() {
            return Ok(());
        }
        let shot_count = self.count_session_shots(session_id)?;
        self.end_session_with_state(session_id, recovery_state::SAFELY_CLOSED)?;
        if let Some(ref entry_id) = session.entry_id {
            let status = if shot_count > 0 {
                entry_status::DONE
            } else {
                entry_status::WAITING
            };
            let _ = self.set_entry_status(entry_id, status);
        }
        let _ = self.maybe_save_training_history(
            session_id,
            session.competition_id.is_none(),
        )?;
        Ok(())
    }

    /// Alias kept for older call sites / tests.
    pub fn abandon_session(&mut self, session_id: &str) -> Result<(), String> {
        self.close_interrupted_session(session_id)
    }

    /// Events for support dump (optional sessions filter; empty = all open/interrupted).
    pub fn dump_events_jsonl(&self, session_ids: &[String]) -> Result<String, String> {
        let ids = if session_ids.is_empty() {
            self.list_unclean_sessions()?
        } else {
            session_ids.to_vec()
        };
        if ids.is_empty() {
            return Ok(String::new());
        }
        let placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, session_id, sequence, kind, created_at, payload, actor_type
             FROM events WHERE session_id IN ({placeholders})
             ORDER BY session_id, sequence"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params_vec: Vec<&dyn rusqlite::types::ToSql> = ids
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt
            .query_map(params_vec.as_slice(), |r| {
                let payload_str: String = r.get(5)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
                Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "sessionId": r.get::<_, String>(1)?,
                    "sequence": r.get::<_, i64>(2)?,
                    "kind": r.get::<_, String>(3)?,
                    "createdAt": r.get::<_, String>(4)?,
                    "payload": payload,
                    "actorType": r.get::<_, String>(6)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut lines = String::new();
        for r in rows {
            let v = r.map_err(|e| e.to_string())?;
            lines.push_str(&v.to_string());
            lines.push('\n');
        }
        Ok(lines)
    }

    /// Consistent DB snapshot via VACUUM INTO (safe while WAL is open).
    pub fn vacuum_into(&self, dest: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if dest.exists() {
            std::fs::remove_file(dest).map_err(|e| e.to_string())?;
        }
        let dest_str = dest
            .to_str()
            .ok_or_else(|| "Ungültiger Export-Pfad".to_string())?;
        // Checkpoint first so WAL content is durable.
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        self.conn
            .execute("VACUUM INTO ?1", params![dest_str])
            .map_err(|e| format!("VACUUM INTO: {e}"))?;
        Ok(())
    }
}
