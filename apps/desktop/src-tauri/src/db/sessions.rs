use super::{event_kind, Database};
use chrono::Utc;
use rusqlite::params;
use serde_json::Value;
use uuid::Uuid;

/// Session lifecycle for Recovery Gate (persisted in `sessions.recovery_state`).
pub mod recovery_state {
    pub const ACTIVE: &str = "active";
    pub const INTERRUPTED: &str = "interrupted";
    pub const RECOVERED: &str = "recovered";
    pub const SAFELY_CLOSED: &str = "safely_closed";
    pub const CLEAN: &str = "clean";
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub shooter_name: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub competition_id: Option<String>,
    pub entry_id: Option<String>,
    pub person_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredEvent {
    pub id: String,
    pub session_id: String,
    pub sequence: i64,
    pub kind: String,
    pub created_at: String,
    pub payload: Value,
    pub actor_type: String,
}

impl Database {
    pub fn start_session(
        &mut self,
        shooter_name: &str,
        competition_id: Option<&str>,
        entry_id: Option<&str>,
        person_id: Option<&str>,
    ) -> Result<SessionInfo, String> {
        let id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO sessions
                 (id, shooter_name, started_at, competition_id, entry_id, person_id,
                  next_sequence, recovery_state, last_autosave_at, last_autosave_sequence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, 0)",
                params![
                    id,
                    shooter_name,
                    started_at,
                    competition_id,
                    entry_id,
                    person_id,
                    recovery_state::ACTIVE,
                    started_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        let event = self.append_event(
            &id,
            event_kind::SESSION_STARTED,
            "system",
            serde_json::json!({
                "shooterName": shooter_name,
                "competitionId": competition_id,
                "entryId": entry_id,
                "personId": person_id,
            }),
        )?;
        self.touch_autosave(&id, Some(event.sequence))?;
        let info = SessionInfo {
            id,
            shooter_name: shooter_name.to_string(),
            started_at,
            ended_at: None,
            competition_id: competition_id.map(str::to_string),
            entry_id: entry_id.map(str::to_string),
            person_id: person_id.map(str::to_string),
        };
        // WAL-safe VACUUM INTO snapshot (best-effort; never blocks session start).
        self.try_session_boundary_snapshot(&info.id);
        Ok(info)
    }

    pub fn end_session(&mut self, session_id: &str) -> Result<(), String> {
        self.end_session_with_state(session_id, recovery_state::CLEAN)
    }

    /// Close an open session with an explicit recovery outcome (`clean` / `safely_closed`).
    pub fn end_session_with_state(
        &mut self,
        session_id: &str,
        state: &str,
    ) -> Result<(), String> {
        let ended_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE sessions
                 SET ended_at = ?1, recovery_state = ?2,
                     last_autosave_at = ?1
                 WHERE id = ?3 AND ended_at IS NULL",
                params![ended_at, state, session_id],
            )
            .map_err(|e| e.to_string())?;
        self.append_event(
            session_id,
            event_kind::SESSION_ENDED,
            "system",
            serde_json::json!({}),
        )?;
        // WAL-safe VACUUM INTO snapshot after clean close (best-effort).
        self.try_session_boundary_snapshot(session_id);
        Ok(())
    }

    /// Update autosave marker (same writer path; used from ingest TX via `touch_autosave_in_tx`).
    pub fn touch_autosave(
        &mut self,
        session_id: &str,
        sequence: Option<i64>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        if let Some(seq) = sequence {
            self.conn
                .execute(
                    "UPDATE sessions
                     SET last_autosave_at = ?1,
                         last_autosave_sequence = ?2,
                         recovery_state = CASE
                           WHEN recovery_state IN ('clean', 'safely_closed') THEN recovery_state
                           WHEN recovery_state = 'recovered' THEN 'active'
                           ELSE 'active'
                         END
                     WHERE id = ?3 AND ended_at IS NULL",
                    params![now, seq, session_id],
                )
                .map_err(|e| e.to_string())?;
        } else {
            // Heartbeat: refresh timestamp only (keep sequence).
            self.conn
                .execute(
                    "UPDATE sessions
                     SET last_autosave_at = ?1,
                         recovery_state = CASE
                           WHEN recovery_state IN ('clean', 'safely_closed') THEN recovery_state
                           WHEN recovery_state = 'recovered' THEN 'active'
                           ELSE COALESCE(recovery_state, 'active')
                         END
                     WHERE id = ?2 AND ended_at IS NULL",
                    params![now, session_id],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn mark_session_recovered(&mut self, session_id: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE sessions
                 SET recovery_state = ?1, last_autosave_at = ?2
                 WHERE id = ?3 AND ended_at IS NULL",
                params![recovery_state::RECOVERED, now, session_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn append_event(
        &mut self,
        session_id: &str,
        kind: &str,
        actor_type: &str,
        payload: Value,
    ) -> Result<StoredEvent, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let event = append_event_in_tx(&tx, session_id, kind, actor_type, payload, None)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(event)
    }
}

/// Allocate the next session event sequence inside an open transaction.
/// Single writer for `sessions.next_sequence` — Arena and session lifecycle share this.
pub fn allocate_sequence(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<i64, String> {
    let next: i64 = tx
        .query_row(
            "SELECT next_sequence FROM sessions WHERE id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("session missing for sequence: {e}"))?;
    tx.execute(
        "UPDATE sessions SET next_sequence = ?1 WHERE id = ?2",
        params![next + 1, session_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(next)
}

/// Insert one event row inside an open transaction (uses [`allocate_sequence`]).
pub fn append_event_in_tx(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    kind: &str,
    actor_type: &str,
    payload: Value,
    parser_version: Option<&str>,
) -> Result<StoredEvent, String> {
    let sequence = allocate_sequence(tx, session_id)?;
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO events
         (id, session_id, sequence, kind, created_at, payload, actor_type, parser_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            session_id,
            sequence,
            kind,
            created_at,
            payload.to_string(),
            actor_type,
            parser_version
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(StoredEvent {
        id,
        session_id: session_id.to_string(),
        sequence,
        kind: kind.to_string(),
        created_at,
        payload,
        actor_type: actor_type.to_string(),
    })
}

/// Write autosave marker inside an open ingest transaction (no extra I/O round-trip).
pub fn touch_autosave_in_tx(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    sequence: i64,
    at: &str,
) -> Result<(), String> {
    tx.execute(
        "UPDATE sessions
         SET last_autosave_at = ?1,
             last_autosave_sequence = ?2,
             recovery_state = CASE
               WHEN recovery_state IN ('clean', 'safely_closed') THEN recovery_state
               WHEN recovery_state = 'recovered' THEN 'active'
               ELSE 'active'
             END
         WHERE id = ?3 AND ended_at IS NULL",
        params![at, sequence, session_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
