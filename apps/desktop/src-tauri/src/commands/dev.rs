//! Developer / admin diagnostics — not for normal range operation.

use crate::arena::PARSER_VERSION;
use crate::commands::AdminSession;
use crate::db::event_kind;
use crate::engine::{LiveState, StandEngine};
use std::sync::Arc;
use tauri::AppHandle;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevDiagnostics {
    pub db_path: String,
    pub schema_version: i64,
    pub has_competition_id: bool,
    pub has_entry_id: bool,
    pub parser_version: String,
    pub session_id: Option<String>,
    pub session_shots: i64,
    pub total_shots: i64,
    pub total_frames: i64,
    pub shot_received_events: i64,
    pub unclean_sessions: Vec<String>,
    pub recent_shots: Vec<serde_json::Value>,
    pub live_running: bool,
    pub live_ui_shots: usize,
}

#[tauri::command]
pub fn dev_diagnostics(engine: tauri::State<'_, Arc<StandEngine>>) -> Result<DevDiagnostics, String> {
    let snap = engine.snapshot();
    engine.with_db(|db| {
        let session_id = snap.session.as_ref().map(|s| s.id.clone());
        let session_shots = match &session_id {
            Some(id) => db.count_session_shots(id)?,
            None => 0,
        };
        let recent = match &session_id {
            Some(id) => db.list_recent_shots(id, 8)?,
            None => Vec::new(),
        };
        Ok(DevDiagnostics {
            db_path: db.path().display().to_string(),
            schema_version: db.schema_version()?,
            has_competition_id: db.session_has_column("competition_id"),
            has_entry_id: db.session_has_column("entry_id"),
            parser_version: PARSER_VERSION.to_string(),
            session_id,
            session_shots,
            total_shots: db.count_all_shots()?,
            total_frames: db.count_frames()?,
            shot_received_events: db.count_events_kind(event_kind::SHOT_RECEIVED)?,
            unclean_sessions: db.list_unclean_sessions()?,
            recent_shots: recent,
            live_running: engine.is_running(),
            live_ui_shots: snap.shots.len(),
        })
    })
}

/// Inject a test shot through Arena ingest and verify it lands in SQLite + UI.
/// Writes real shot data, so it requires the server-side admin unlock.
#[tauri::command]
pub fn dev_inject_test_shot(
    app: AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    session: tauri::State<'_, AdminSession>,
    x: Option<i32>,
    y: Option<i32>,
) -> Result<LiveState, String> {
    session.require()?;
    engine.fire_aim_shot(&app, f64::from(x.unwrap_or(40)), f64::from(y.unwrap_or(-25)))
}
