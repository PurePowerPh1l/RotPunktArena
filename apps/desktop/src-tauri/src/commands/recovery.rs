//! Recovery gate + diagnostics export — driven by session autosave markers.

use crate::db::RecoverySessionInfo;
use crate::engine::{LiveState, StandEngine};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Manager;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyExportResult {
    pub path: String,
    pub unclean_session_ids: Vec<String>,
    pub schema_version: i64,
}

#[tauri::command]
pub fn list_recovery_sessions(
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<Vec<RecoverySessionInfo>, String> {
    let mut list = engine.with_db(|db| db.list_recovery_sessions())?;
    // Active (resumed) session stays open until end — hide it from the gate.
    if let Some(active_id) = engine
        .snapshot()
        .session
        .filter(|s| s.ended_at.is_none())
        .map(|s| s.id)
    {
        list.retain(|s| s.id != active_id);
    }
    Ok(list)
}

#[tauri::command]
pub fn close_interrupted_session(
    engine: tauri::State<'_, Arc<StandEngine>>,
    session_id: String,
) -> Result<LiveState, String> {
    engine.close_interrupted_session(&session_id)
}

/// Legacy alias — same as `close_interrupted_session`.
#[tauri::command]
pub fn abandon_session(
    engine: tauri::State<'_, Arc<StandEngine>>,
    session_id: String,
) -> Result<LiveState, String> {
    engine.close_interrupted_session(&session_id)
}

#[tauri::command]
pub fn resume_session(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    session_id: String,
    use_simulator: Option<bool>,
) -> Result<LiveState, String> {
    engine.resume_session(app, &session_id, use_simulator.unwrap_or(true))
}

#[tauri::command]
pub fn export_diagnostics(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    path: Option<String>,
) -> Result<EmergencyExportResult, String> {
    export_emergency_bundle_inner(app, engine, path)
}

/// Legacy alias — same as `export_diagnostics`.
#[tauri::command]
pub fn export_emergency_bundle(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    path: Option<String>,
) -> Result<EmergencyExportResult, String> {
    export_emergency_bundle_inner(app, engine, path)
}

fn export_emergency_bundle_inner(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    path: Option<String>,
) -> Result<EmergencyExportResult, String> {
    let unclean = engine.with_db(|db| db.list_unclean_sessions())?;
    let schema_version = engine.with_db(|db| db.schema_version())?;
    let db_path = engine.with_db(|db| db.path().to_path_buf());

    let zip_path = resolve_export_path(&app, path)?;
    if let Some(parent) = zip_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let staging = zip_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!(
            ".reddot-export-{}.sqlite",
            chrono::Utc::now().timestamp_millis()
        ));

    engine.with_db(|db| db.vacuum_into(&staging))?;

    let events_jsonl = engine.with_db(|db| db.dump_events_jsonl(&unclean))?;

    let manifest = serde_json::json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "schemaVersion": schema_version,
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "dbSourcePath": db_path.to_string_lossy(),
        "uncleanSessionIds": unclean,
        "parserVersion": crate::PARSER_VERSION,
    });

    write_emergency_zip(&zip_path, &staging, &manifest, &events_jsonl)?;
    let _ = std::fs::remove_file(&staging);

    Ok(EmergencyExportResult {
        path: zip_path.to_string_lossy().into_owned(),
        unclean_session_ids: unclean,
        schema_version,
    })
}

fn resolve_export_path(app: &tauri::AppHandle, path: Option<String>) -> Result<PathBuf, String> {
    if let Some(p) = path.filter(|s| !s.trim().is_empty()) {
        let pb = PathBuf::from(p);
        if pb.extension().and_then(|e| e.to_str()) == Some("zip") {
            return Ok(pb);
        }
        return Ok(pb.with_extension("zip"));
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let exports = data_dir.join("exports");
    std::fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    Ok(exports.join(format!("reddot-diagnostics-{stamp}.zip")))
}

fn write_emergency_zip(
    zip_path: &Path,
    sqlite_copy: &Path,
    manifest: &serde_json::Value,
    events_jsonl: &str,
) -> Result<(), String> {
    let file = File::create(zip_path).map_err(|e| format!("ZIP anlegen: {e}"))?;
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("reddot.sqlite", opts)
        .map_err(|e| e.to_string())?;
    let mut db_file = File::open(sqlite_copy).map_err(|e| e.to_string())?;
    std::io::copy(&mut db_file, &mut zip).map_err(|e| e.to_string())?;

    zip.start_file("manifest.json", opts)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        serde_json::to_string_pretty(manifest)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;

    if !events_jsonl.is_empty() {
        zip.start_file("events.jsonl", opts)
            .map_err(|e| e.to_string())?;
        zip.write_all(events_jsonl.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
