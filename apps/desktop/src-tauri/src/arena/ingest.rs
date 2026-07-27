//! Named ingest steps inside one fail-closed transaction.
//! Orchestrator: [`super::Database::ingest_raw_frame`].

use super::{AcceptedShot, IngestOutcome, PARSER_VERSION};
use crate::db::{
    append_event_in_tx, count_scored_shots_for_limit, event_kind, session_effective_max_shots,
};
use crate::protocol::Shot;
use chrono::Utc;
use rusqlite::{params, Transaction};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// True iff `sessions` row exists and `ended_at IS NULL` (same TX as ingest persist).
pub(crate) fn session_open_for_ingest(
    tx: &Transaction<'_>,
    session_id: &str,
) -> Result<bool, String> {
    let ended_at: Option<Option<String>> = tx
        .query_row(
            "SELECT ended_at FROM sessions WHERE id = ?1",
            params![session_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional_err()?;
    match ended_at {
        Some(None) => Ok(true),
        Some(Some(_)) | None => Ok(false),
    }
}

/// Dedupe: prefer device_sequence row, else SHA of raw frame.
/// Returns `Some(Duplicate)` if already persisted; `None` if new.
pub(crate) fn dedupe(
    tx: &Transaction<'_>,
    session_id: &str,
    device_sequence: Option<i64>,
    frame_sha: &str,
) -> Result<Option<IngestOutcome>, String> {
    if let Some(seq) = device_sequence {
        let existing: Option<(String,)> = tx
            .query_row(
                "SELECT id FROM frames WHERE session_id = ?1 AND device_sequence = ?2",
                params![session_id, seq],
                |r| Ok((r.get(0)?,)),
            )
            .optional_err()?;
        if let Some((fid,)) = existing {
            return Ok(Some(IngestOutcome::Duplicate {
                frame_sha256: frame_sha.to_string(),
                existing_frame_id: fid,
            }));
        }
    }

    let existing_sha: Option<(String,)> = tx
        .query_row(
            "SELECT id FROM frames WHERE session_id = ?1 AND frame_sha256 = ?2",
            params![session_id, frame_sha],
            |r| Ok((r.get(0)?,)),
        )
        .optional_err()?;
    if let Some((fid,)) = existing_sha {
        return Ok(Some(IngestOutcome::Duplicate {
            frame_sha256: frame_sha.to_string(),
            existing_frame_id: fid,
        }));
    }

    Ok(None)
}

/// Persist raw frame with `parse_status = 'pending'`. Returns new `frame_id`.
pub(crate) fn persist_frame(
    tx: &Transaction<'_>,
    session_id: &str,
    received_at: &str,
    raw_hex: &str,
    frame_sha: &str,
    device_sequence: Option<i64>,
) -> Result<String, String> {
    let frame_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO frames
         (id, session_id, received_at, raw_frame_hex, frame_sha256, parser_version, parse_status, device_sequence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            frame_id,
            session_id,
            received_at,
            raw_hex,
            frame_sha,
            PARSER_VERSION,
            "pending",
            device_sequence
        ],
    )
    .map_err(|e| format!("insert frame: {e}"))?;
    Ok(frame_id)
}

/// Frame stored but parse failed — mark error + `frame_parse_error` event.
pub(crate) fn reject_parse(
    tx: &Transaction<'_>,
    session_id: &str,
    actor: &str,
    frame_id: &str,
    frame_sha: &str,
    err: String,
) -> Result<IngestOutcome, String> {
    tx.execute(
        "UPDATE frames SET parse_status = 'error' WHERE id = ?1",
        params![frame_id],
    )
    .map_err(|e| e.to_string())?;
    let _event = append_event_in_tx(
        tx,
        session_id,
        event_kind::FRAME_PARSE_ERROR,
        actor,
        serde_json::json!({
            "frameId": frame_id,
            "error": &err,
            "frameSha256": frame_sha,
        }),
        Some(PARSER_VERSION),
    )?;
    Ok(IngestOutcome::ParseFailed {
        frame_id: frame_id.to_string(),
        error: err,
    })
}

/// Competition max_shots: refuse further scored shots once the per-session limit is reached.
/// Count is session-scoped (each series / Nachkauf gets a fresh `max_shots` budget).
/// Returns `Some(LimitReached)` when rejected; `None` when under the limit (or no limit).
pub(crate) fn reject_limit(
    tx: &Transaction<'_>,
    session_id: &str,
    actor: &str,
    frame_id: &str,
) -> Result<Option<IngestOutcome>, String> {
    let Some(max) = session_effective_max_shots(tx, session_id)? else {
        return Ok(None);
    };
    let n = count_scored_shots_for_limit(tx, session_id)?;
    if n < max {
        return Ok(None);
    }

    tx.execute(
        "UPDATE frames SET parse_status = 'rejected_limit' WHERE id = ?1",
        params![frame_id],
    )
    .map_err(|e| e.to_string())?;
    let _event = append_event_in_tx(
        tx,
        session_id,
        event_kind::SHOT_REJECTED_LIMIT,
        actor,
        serde_json::json!({
            "frameId": frame_id,
            "maxShots": max,
            "currentShots": n,
        }),
        Some(PARSER_VERSION),
    )?;
    Ok(Some(IngestOutcome::LimitReached {
        max_shots: max,
        current_shots: n,
    }))
}

/// Accept scored shot: event → shots projection → frame ok → autosave marker.
pub(crate) fn accept(
    tx: &Transaction<'_>,
    session_id: &str,
    actor: &str,
    frame_id: &str,
    frame_sha: &str,
    received_at: &str,
    device_sequence: Option<i64>,
    shot: &Shot,
) -> Result<AcceptedShot, String> {
    let shot_index = count_shots(tx, session_id)? + 1;
    let series_total = sum_scores(tx, session_id)? + shot.value_display;
    let series_teiler_total = sum_teiler(tx, session_id)? + shot.distance_display;
    let classification = "scored";
    let shot_row_id = Uuid::new_v4().to_string();

    let payload = serde_json::json!({
        "frameId": frame_id,
        "frameSha256": frame_sha,
        "shotIndex": shot_index,
        "valueRaw": shot.value_raw,
        "distanceRaw": shot.distance_raw,
        "x": shot.x,
        "y": shot.y,
        "valueDisplay": shot.value_display,
        "distanceDisplay": shot.distance_display,
        "classification": classification,
        "deviceSequence": device_sequence,
    });

    let event = append_event_in_tx(
        tx,
        session_id,
        event_kind::SHOT_RECEIVED,
        actor,
        payload,
        Some(PARSER_VERSION),
    )
    .map_err(|e| format!("insert event: {e}"))?;
    let sequence = event.sequence;
    let event_id = event.id.clone();

    tx.execute(
        "INSERT INTO shots
         (id, session_id, frame_id, session_sequence, shot_index, score,
          value_raw, distance_raw, x, y, classification, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            shot_row_id,
            session_id,
            frame_id,
            sequence,
            shot_index,
            shot.value_display,
            shot.value_raw,
            shot.distance_raw,
            shot.x,
            shot.y,
            classification,
            received_at
        ],
    )
    .map_err(|e| format!("insert shot projection: {e}"))?;

    tx.execute(
        "UPDATE frames SET parse_status = 'ok' WHERE id = ?1",
        params![frame_id],
    )
    .map_err(|e| e.to_string())?;

    // Autosave marker in the same TX — Recovery Gate heartbeat (no second write path).
    crate::db::touch_autosave_in_tx(tx, session_id, sequence, received_at)?;

    Ok(AcceptedShot {
        frame_id: frame_id.to_string(),
        event_id,
        session_sequence: sequence,
        shot_index,
        score: shot.value_display,
        value_raw: shot.value_raw,
        distance_raw: shot.distance_raw,
        x: shot.x,
        y: shot.y,
        classification: classification.to_string(),
        series_total,
        series_teiler_total,
        frame_sha256: frame_sha.to_string(),
        parser_version: PARSER_VERSION.to_string(),
    })
}

/// Content hash for UNIQUE(session_id, frame_sha256). Device sequence is a separate dedupe path.
pub(crate) fn frame_content_sha(raw: &[u8]) -> String {
    if !raw.is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(raw);
        return hex::encode(hasher.finalize());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"empty-frame-fallback");
    hasher.update(Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
    hex::encode(hasher.finalize())
}

fn count_shots(tx: &Transaction<'_>, session_id: &str) -> Result<i32, String> {
    let n: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM shots WHERE session_id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n as i32)
}

fn sum_scores(tx: &Transaction<'_>, session_id: &str) -> Result<f64, String> {
    let n: f64 = tx
        .query_row(
            "SELECT COALESCE(SUM(score), 0) FROM shots WHERE session_id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n)
}

fn sum_teiler(tx: &Transaction<'_>, session_id: &str) -> Result<f64, String> {
    let n: f64 = tx
        .query_row(
            "SELECT COALESCE(SUM(CAST(distance_raw AS REAL) / 10.0), 0)
             FROM shots WHERE session_id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n)
}

trait OptionalQuery<T> {
    fn optional_err(self) -> Result<Option<T>, String>;
}

impl<T> OptionalQuery<T> for Result<T, rusqlite::Error> {
    fn optional_err(self) -> Result<Option<T>, String> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}
