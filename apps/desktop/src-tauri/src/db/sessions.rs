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
    #[serde(default)]
    pub simulated: bool,
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
    pub fn finish_probe_phase(&self, session_id: &str, probe_shots: i64) -> Result<(), String> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(|e| e.to_string())?;
        let changed = tx.execute(
            "UPDATE sessions SET phase = 'match' WHERE id = ?1 AND phase = 'probe' AND ended_at IS NULL",
            params![session_id],
        ).map_err(|e| e.to_string())?;
        if changed != 1 {
            return Err("Keine offene Probephase".into());
        }
        append_event_in_tx(
            &tx,
            session_id,
            event_kind::PROBE_FINISHED,
            "operator",
            serde_json::json!({ "probeShots": probe_shots }),
            None,
        )?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn start_session(
        &mut self,
        shooter_name: &str,
        competition_id: Option<&str>,
        entry_id: Option<&str>,
        person_id: Option<&str>,
    ) -> Result<SessionInfo, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let info = Self::insert_session_in_tx(
            &tx,
            shooter_name,
            competition_id,
            entry_id,
            person_id,
            None,
            super::session_phase::MATCH,
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        self.spawn_session_boundary_snapshot(&info.id);
        Ok(info)
    }

    pub(super) fn insert_session_in_tx(
        tx: &rusqlite::Transaction<'_>,
        shooter_name: &str,
        competition_id: Option<&str>,
        entry_id: Option<&str>,
        person_id: Option<&str>,
        max_shots: Option<i64>,
        phase: &str,
    ) -> Result<SessionInfo, String> {
        let id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();
        tx.execute(
                "INSERT INTO sessions
                 (id, shooter_name, started_at, competition_id, entry_id, person_id,
                  next_sequence, recovery_state, last_autosave_at, last_autosave_sequence, max_shots, phase)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, 0, ?9, ?10)",
                params![
                    id,
                    shooter_name,
                    started_at,
                    competition_id,
                    entry_id,
                    person_id,
                    recovery_state::ACTIVE,
                    started_at,
                    max_shots,
                    phase,
                ],
            )
            .map_err(|e| e.to_string())?;
        tx.execute("UPDATE sessions SET rules_origin='captured', rules_json =
            (SELECT json_object('version', 1, 'tenthsEnabled', COALESCE(c.tenths_enabled, 1),
             'maxShots', COALESCE(?2, c.max_shots), 'scoringMode', COALESCE(c.scoring_mode, 'ringe'),
             'nachkaufEnabled', COALESCE(c.nachkauf_enabled, 0),
             'teamScoringEnabled', COALESCE(c.team_scoring_enabled, 0), 'teamCount', COALESCE(c.team_count, 3),
             'probeEnabled', COALESCE(c.probe_enabled, 0), 'discipline', COALESCE(c.discipline, 'training'))
             FROM (SELECT 1) LEFT JOIN competitions c ON c.id = ?3) WHERE id = ?1",
            params![id, max_shots, competition_id]).map_err(|e| e.to_string())?;
        let event = append_event_in_tx(
            tx,
            &id,
            event_kind::SESSION_STARTED,
            "system",
            serde_json::json!({
                "shooterName": shooter_name,
                "competitionId": competition_id,
                "entryId": entry_id,
                "personId": person_id,
            }),
            None,
        )?;
        touch_autosave_in_tx(tx, &id, event.sequence, &started_at)?;
        let info = SessionInfo {
            simulated: false,
            id,
            shooter_name: shooter_name.to_string(),
            started_at,
            ended_at: None,
            competition_id: competition_id.map(str::to_string),
            entry_id: entry_id.map(str::to_string),
            person_id: person_id.map(str::to_string),
        };
        Ok(info)
    }

    /// Persist the effective per-session shot limit (`NULL` = unlimited /
    /// endless) so Arena ingest can enforce it inside the same TX.
    pub fn set_session_max_shots(
        &mut self,
        session_id: &str,
        max_shots: Option<i64>,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE sessions SET max_shots = ?1 WHERE id = ?2",
                params![max_shots, session_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_session_max_shots(&self, session_id: &str) -> Result<Option<i64>, String> {
        self.conn
            .query_row(
                "SELECT max_shots FROM sessions WHERE id = ?1",
                params![session_id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .map_err(|e| e.to_string())
    }

    /// Persist the session phase (`probe` / `match`) so Arena ingest can
    /// classify shots inside the same TX.
    pub fn set_session_phase(&mut self, session_id: &str, phase: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE sessions SET phase = ?1 WHERE id = ?2",
                params![phase, session_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Current session phase; legacy rows without the column value → `match`.
    pub fn get_session_phase(&self, session_id: &str) -> Result<String, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(phase, 'match') FROM sessions WHERE id = ?1",
                params![session_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn end_session(&mut self, session_id: &str) -> Result<(), String> {
        self.end_session_with_state(session_id, recovery_state::CLEAN)
    }

    /// Finish once: event and any dependent changes share the same transaction.
    pub(super) fn finish_session<T>(
        &self,
        session_id: &str,
        state: &str,
        after_close: impl FnOnce(&Self) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let ended_at = Utc::now().to_rfc3339();
        let changed = tx
            .execute(
                "UPDATE sessions SET ended_at = ?1, recovery_state = ?2, last_autosave_at = ?1
             WHERE id = ?3 AND ended_at IS NULL",
                params![ended_at, state, session_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            if self.get_session(session_id)?.is_none() {
                return Err("Session nicht gefunden".into());
            }
            return Ok(None);
        }
        append_event_in_tx(
            &tx,
            session_id,
            event_kind::SESSION_ENDED,
            "system",
            serde_json::json!({}),
            None,
        )?;
        let result = after_close(self)?;
        tx.commit().map_err(|e| e.to_string())?;
        self.spawn_session_boundary_snapshot(session_id);
        Ok(Some(result))
    }

    pub fn end_session_with_state(&mut self, session_id: &str, state: &str) -> Result<(), String> {
        self.finish_session(session_id, state, |_| Ok(()))?;
        Ok(())
    }

    pub fn finish_live_session(
        &self,
        session_id: &str,
        entry_id: Option<&str>,
        is_training: bool,
        endless: bool,
        shot_count: i64,
    ) -> Result<super::TrainingSaveInfo, String> {
        let saved = self.finish_session(session_id, recovery_state::CLEAN, |db| {
            if let Some(entry_id) = entry_id {
                db.set_entry_status(entry_id, super::entry_status::DONE)?;
            }
            if endless && is_training {
                return Ok(super::TrainingSaveInfo {
                    saved: false,
                    shot_count,
                    min_shots: super::TRAINING_HISTORY_MIN_SHOTS,
                    reason: "endless".into(),
                });
            }
            db.maybe_save_training_history(session_id, is_training)
        })?;
        match saved {
            Some(info) => Ok(info),
            None if endless && is_training => Ok(super::TrainingSaveInfo {
                saved: false,
                shot_count,
                min_shots: super::TRAINING_HISTORY_MIN_SHOTS,
                reason: "endless".into(),
            }),
            None if !is_training => Ok(super::TrainingSaveInfo::not_training()),
            None => {
                let (saved, max_shots): (i64, Option<i64>) = self
                    .conn
                    .query_row(
                        "SELECT training_saved, max_shots FROM sessions WHERE id = ?1",
                        params![session_id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .map_err(|e| e.to_string())?;
                let shot_count = self.count_session_shots(session_id)?;
                let min_shots = max_shots
                    .filter(|&n| n > 0)
                    .map(super::normalize_training_series_shots)
                    .unwrap_or(super::TRAINING_HISTORY_MIN_SHOTS);
                Ok(super::TrainingSaveInfo {
                    saved: saved != 0,
                    shot_count,
                    min_shots,
                    reason: if saved != 0 {
                        "saved"
                    } else {
                        "already_closed"
                    }
                    .into(),
                })
            }
        }
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

/// Session phase inside an open transaction (Arena ingest classification).
pub fn session_phase_in_tx(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<String, String> {
    tx.query_row(
        "SELECT COALESCE(phase, 'match') FROM sessions WHERE id = ?1",
        params![session_id],
        |r| r.get(0),
    )
    .map_err(|e| format!("session phase: {e}"))
}

/// Allocate the next session event sequence inside an open transaction.
/// Single writer for `sessions.next_sequence` — Arena and session lifecycle share this.
pub fn allocate_sequence(tx: &rusqlite::Transaction<'_>, session_id: &str) -> Result<i64, String> {
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

#[cfg(test)]
mod finish_tests {
    use super::*;

    #[test]
    fn probe_transition_rolls_back_phase_and_event_then_accepts_first_scored_shot() {
        let mut db = Database::open_in_memory().unwrap();
        let session = db.start_session("Probe fixture", None, None, None).unwrap();
        db.set_session_phase(&session.id, super::super::session_phase::PROBE)
            .unwrap();
        let mut frame =
            crate::protocol::build_synthetic_shot_frame("10.0", "001.00", "00001", "00002")
                .unwrap();
        assert!(matches!(
            db.ingest_raw_frame(&session.id, &frame, "test", None)
                .unwrap(),
            crate::arena::IngestOutcome::Accepted(_)
        ));
        db.conn.execute_batch("CREATE TRIGGER fail_probe BEFORE INSERT ON events WHEN NEW.kind = 'probe_finished' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(db.finish_probe_phase(&session.id, 1).is_err());
        assert_eq!(db.get_session_phase(&session.id).unwrap(), "probe");
        assert_eq!(db.count_events_kind(event_kind::PROBE_FINISHED).unwrap(), 0);
        db.conn.execute_batch("DROP TRIGGER fail_probe").unwrap();
        db.finish_probe_phase(&session.id, 1).unwrap();
        assert!(db.finish_probe_phase(&session.id, 1).is_err());
        crate::protocol::stamp_frame_nonce(&mut frame);
        let crate::arena::IngestOutcome::Accepted(accepted) = db
            .ingest_raw_frame(&session.id, &frame, "test", None)
            .unwrap()
        else {
            panic!("first scored shot rejected")
        };
        assert_eq!(accepted.shot_index, 1);
        assert_eq!(accepted.series_total, 10.0);
        assert_eq!(
            db.load_session_ui_shots(&session.id, "probe")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.load_session_ui_shots(&session.id, "scored")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(db.count_events_kind(event_kind::PROBE_FINISHED).unwrap(), 1);
    }

    #[test]
    fn failed_dependent_write_rolls_back_close_and_event() {
        let mut db = super::super::Database::open_in_memory().unwrap();
        let session = db.start_session("Rollback", None, None, None).unwrap();
        let before = db.count_events_kind(event_kind::SESSION_ENDED).unwrap();
        let result: Result<Option<()>, String> =
            db.finish_session(&session.id, recovery_state::CLEAN, |_| {
                Err("injected failure".into())
            });
        assert!(result.is_err());
        assert!(db
            .get_session(&session.id)
            .unwrap()
            .unwrap()
            .ended_at
            .is_none());
        assert_eq!(
            db.count_events_kind(event_kind::SESSION_ENDED).unwrap(),
            before
        );
        db.end_session(&session.id).unwrap();
        assert_eq!(
            db.count_events_kind(event_kind::SESSION_ENDED).unwrap(),
            before + 1
        );
    }
}
