//! Arena Core — atomic frame ingest (fail-closed before UI).

mod ingest;

use crate::db::Database;
use crate::protocol::Shot;
use chrono::Utc;
use rusqlite::params;

/// Bump when STX layout / scoring interpretation changes.
pub const PARSER_VERSION: &str = "reddot-stx-v1";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedShot {
    pub frame_id: String,
    pub event_id: String,
    pub session_sequence: i64,
    pub shot_index: i32,
    pub score: f64,
    pub value_raw: i32,
    pub distance_raw: i32,
    pub x: i32,
    pub y: i32,
    pub classification: String,
    pub series_total: f64,
    pub series_teiler_total: f64,
    pub frame_sha256: String,
    pub parser_version: String,
}

#[derive(Debug, Clone)]
pub enum IngestOutcome {
    /// New scored shot — safe to emit to UI after this returns.
    Accepted(AcceptedShot),
    /// Same frame already persisted for this session — no UI emit.
    Duplicate {
        frame_sha256: String,
        existing_frame_id: String,
    },
    /// Frame stored but parse failed — no UI shot.
    ParseFailed {
        frame_id: String,
        error: String,
    },
    /// Competition max_shots already reached — frame stored, no scored shot.
    LimitReached {
        max_shots: i64,
        current_shots: i64,
    },
    /// Session missing or `ended_at` set — no frames/shots/events written.
    SessionInactive {
        session_id: String,
    },
}

impl Database {
    /// Single writer path: save frame → dedupe → parse → event → projection → sequence++.
    /// UI must emit **only** on `IngestOutcome::Accepted` (after COMMIT).
    pub fn ingest_raw_frame(
        &mut self,
        session_id: &str,
        raw: &[u8],
        actor_type: &str,
        device_sequence: Option<i64>,
    ) -> Result<IngestOutcome, String> {
        let received_at = Utc::now().to_rfc3339();
        let raw_hex = hex::encode(raw);
        let frame_sha = ingest::frame_content_sha(raw);
        let actor = if actor_type.trim().is_empty() {
            "device"
        } else {
            actor_type
        };

        let tx = self
            .conn
            .transaction()
            .map_err(|e| format!("begin ingest tx: {e}"))?;

        // Last authority: open session in same TX as any later persist (no frames/shots on fail).
        if !ingest::session_open_for_ingest(&tx, session_id)? {
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(IngestOutcome::SessionInactive {
                session_id: session_id.to_string(),
            });
        }

        if let Some(dup) = ingest::dedupe(&tx, session_id, device_sequence, &frame_sha)? {
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(dup);
        }

        let frame_id = ingest::persist_frame(
            &tx,
            session_id,
            &received_at,
            &raw_hex,
            &frame_sha,
            device_sequence,
        )?;

        let mut shot = match crate::protocol::parse_shot_frame(raw) {
            Ok(s) => s,
            Err(err) => {
                let outcome =
                    ingest::reject_parse(&tx, session_id, actor, &frame_id, &frame_sha, err)?;
                tx.commit().map_err(|e| e.to_string())?;
                return Ok(outcome);
            }
        };

        // Whole-ring competitions: floor points before persist / series totals.
        let tenths = crate::db::session_tenths_enabled(&tx, session_id)?;
        shot.value_display =
            crate::protocol::value_display_for_scoring(shot.value_raw, tenths);

        // Probe phase: shots are unscored (`classification = 'probe'`),
        // exempt from the series limit, and never appear in results.
        let probe_phase = crate::db::session_phase_in_tx(&tx, session_id)?
            == crate::db::session_phase::PROBE;

        if !probe_phase {
            if let Some(outcome) = ingest::reject_limit(&tx, session_id, actor, &frame_id)? {
                tx.commit().map_err(|e| e.to_string())?;
                return Ok(outcome);
            }
        }

        let classification = if probe_phase {
            crate::db::shot_classification::PROBE
        } else {
            crate::db::shot_classification::SCORED
        };
        let accepted = ingest::accept(
            &tx,
            session_id,
            actor,
            &frame_id,
            &frame_sha,
            &received_at,
            device_sequence,
            &shot,
            classification,
        )?;
        tx.commit()
            .map_err(|e| format!("commit ingest: {e}"))?;

        // DIAGNOSE-ONLY: stamp commit Instant (poll may take it).
        crate::connection::shot_latency::note_sqlite_committed_at(std::time::Instant::now());

        // Hybrid snapshot (VACUUM INTO) is the caller's responsibility
        // (`try_maybe_snapshot_after_shot`) so device ACK / UI emit are not
        // delayed by a multi-second VACUUM on the ingest path.
        Ok(IngestOutcome::Accepted(accepted))
    }

    pub fn count_session_shots(&self, session_id: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM shots WHERE session_id = ?1",
                params![session_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn count_frames(&self) -> Result<i64, String> {
        self.conn
            .query_row("SELECT COUNT(*) FROM frames", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn count_events_kind(&self, kind: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE kind = ?1",
                params![kind],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn count_all_shots(&self) -> Result<i64, String> {
        self.conn
            .query_row("SELECT COUNT(*) FROM shots", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn schema_version(&self) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn session_has_column(&self, column: &str) -> bool {
        let Ok(mut stmt) = self.conn.prepare("PRAGMA table_info(sessions)") else {
            return false;
        };
        let Ok(rows) = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        }) else {
            return false;
        };
        let found = rows.flatten().any(|n| n == column);
        found
    }

    pub fn list_recent_shots(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT shot_index, score, x, y, session_sequence, frame_id, created_at
                 FROM shots WHERE session_id = ?1
                 ORDER BY session_sequence DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id, limit], |r| {
                Ok(serde_json::json!({
                    "shotIndex": r.get::<_, i32>(0)?,
                    "score": r.get::<_, f64>(1)?,
                    "x": r.get::<_, i32>(2)?,
                    "y": r.get::<_, i32>(3)?,
                    "sessionSequence": r.get::<_, i64>(4)?,
                    "frameId": r.get::<_, String>(5)?,
                    "createdAt": r.get::<_, String>(6)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }
}

#[allow(dead_code)]
pub fn accepted_from_shot(shot: &Shot) -> (f64, i32, i32) {
    (shot.value_display, shot.x, shot.y)
}
