//! Training history + series reset/save commands.

use crate::db::{TrainingSessionDetail, TrainingSessionSummary, TrainingShooterOption};
use crate::engine::{LiveState, StandEngine};
use std::sync::Arc;

#[tauri::command]
pub fn reset_training_series(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<LiveState, String> {
    engine.reset_training_series(app)
}

#[tauri::command]
pub fn set_training_endless(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    endless: bool,
) -> Result<LiveState, String> {
    engine.set_training_endless(&app, endless)
}

#[tauri::command]
pub fn save_training_session(
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<LiveState, String> {
    engine.save_training_session()
}

#[tauri::command]
pub fn list_training_history(
    engine: tauri::State<'_, Arc<StandEngine>>,
    limit: Option<i64>,
    person_id: Option<String>,
    shooter_name: Option<String>,
) -> Result<Vec<TrainingSessionSummary>, String> {
    engine.with_db(|db| {
        db.list_saved_training_sessions(
            limit.unwrap_or(80),
            person_id.as_deref(),
            shooter_name.as_deref(),
        )
    })
}

#[tauri::command]
pub fn get_training_session_detail(
    engine: tauri::State<'_, Arc<StandEngine>>,
    session_id: String,
) -> Result<Option<TrainingSessionDetail>, String> {
    engine.with_db(|db| db.get_training_session_detail(&session_id))
}

#[tauri::command]
pub fn list_training_shooters(
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<Vec<TrainingShooterOption>, String> {
    engine.with_db(|db| db.list_training_shooters())
}

#[tauri::command]
pub fn clear_training_history(
    engine: tauri::State<'_, Arc<StandEngine>>,
    person_id: Option<String>,
    shooter_name: Option<String>,
) -> Result<i64, String> {
    engine.with_db(|db| {
        db.clear_training_history(person_id.as_deref(), shooter_name.as_deref())
    })
}

#[tauri::command]
pub fn promote_training_shooter(
    engine: tauri::State<'_, Arc<StandEngine>>,
    shooter_name: String,
) -> Result<crate::db::PromoteTrainingShooterResult, String> {
    engine.with_db(|db| db.promote_training_shooter(&shooter_name))
}
