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
pub fn resume_session(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    session_id: String,
    use_simulator: Option<bool>,
) -> Result<LiveState, String> {
    engine.resume_session(app, &session_id, use_simulator.unwrap_or(true))
}

#[tauri::command]
pub async fn export_diagnostics(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    file_name: Option<String>,
) -> Result<EmergencyExportResult, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        export_emergency_bundle_inner(&app, &engine, file_name, None)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Legacy alias — same as `export_diagnostics`.
#[tauri::command]
pub async fn export_emergency_bundle(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    file_name: Option<String>,
) -> Result<EmergencyExportResult, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        export_emergency_bundle_inner(&app, &engine, file_name, None)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Versioned full backup: SQLite + canonical device store + browser goals.
#[tauri::command]
pub async fn export_personal_backup(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    session: tauri::State<'_, crate::commands::AdminSession>,
    goals: String,
) -> Result<EmergencyExportResult, String> {
    session.require()?;
    if goals.len() > 1024 * 1024 {
        return Err("Trainingsziele sind zu groß".into());
    }
    let parsed: serde_json::Value = serde_json::from_str(&goals).map_err(|e| e.to_string())?;
    if parsed["formatVersion"] != 2 || !parsed["entries"].is_object() {
        return Err("Unbekanntes Trainingsziele-Format".into());
    }
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let devices =
            serde_json::to_string_pretty(&crate::connection::load_device_store(&directory)?)
                .map_err(|e| e.to_string())?;
        export_emergency_bundle_inner(
            &app,
            &engine,
            Some(format!("rotpunkt-personal-{}.zip", uuid::Uuid::new_v4())),
            Some(vec![
                ("rfcomm_devices.json".into(), devices),
                ("training-goals.json".into(), goals),
            ]),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

fn export_emergency_bundle_inner(
    app: &tauri::AppHandle,
    engine: &StandEngine,
    file_name: Option<String>,
    personal: Option<Vec<(String, String)>>,
) -> Result<EmergencyExportResult, String> {
    let zip_path = resolve_export_path(app, file_name)?;
    let directory = zip_path.parent().ok_or("Export ohne Verzeichnis")?;
    let id = uuid::Uuid::new_v4();
    let staging = directory.join(format!(".export-{id}.sqlite"));
    let temporary = directory.join(format!(".export-{id}.zip"));
    let result = (|| {
        engine.with_db(|db| db.vacuum_into(&staging))?;
        let snapshot = crate::db::Database::open(&staging)?;
        let unclean = snapshot.list_unclean_sessions()?;
        let schema_version = snapshot.schema_version()?;
        let events = snapshot.dump_events_jsonl(&unclean)?;
        let manifest = serde_json::json!({
            "formatVersion":1, "kind":if personal.is_some() {"personalBackup"} else {"diagnostics"}, "containsPersonalData":true, "containsAdminCredentials":true,
            "appVersion":env!("CARGO_PKG_VERSION"), "schemaVersion":schema_version,
            "exportedAt":chrono::Utc::now().to_rfc3339(), "platform":std::env::consts::OS,
            "arch":std::env::consts::ARCH, "uncleanSessionIds":unclean, "parserVersion":crate::PARSER_VERSION,
        });
        drop(snapshot);
        write_emergency_zip(
            &temporary,
            &staging,
            &manifest,
            &events,
            personal.as_deref().unwrap_or(&[]),
        )?;
        // Atomic create-only publication; an earlier export is never truncated.
        std::fs::hard_link(&temporary, &zip_path).map_err(|e| {
            format!("Export veröffentlichen (vorhandene Datei bleibt erhalten): {e}")
        })?;
        Ok(EmergencyExportResult {
            path: zip_path.to_string_lossy().into_owned(),
            unclean_session_ids: unclean,
            schema_version,
        })
    })();
    for path in [
        temporary,
        staging.clone(),
        PathBuf::from(format!("{}-wal", staging.display())),
        PathBuf::from(format!("{}-shm", staging.display())),
    ] {
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Resolve the export ZIP path. The diagnostics bundle contains the full
/// database (incl. PII), so it is always written inside `app_data/exports`.
/// A caller-supplied `file_name` may only choose the file name within that
/// directory — path separators, `..` and absolute paths are rejected so the
/// WebView cannot write the DB to an arbitrary location.
fn resolve_export_path(
    app: &tauri::AppHandle,
    file_name: Option<String>,
) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let exports = data_dir.join("exports");
    std::fs::create_dir_all(&exports).map_err(|e| e.to_string())?;

    let name = match file_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(raw) => {
            if raw.contains(['/', '\\', ':', '<', '>', '|', '?', '*'])
                || raw.contains("..")
                || raw.chars().any(char::is_control)
                || raw.ends_with('.')
            {
                return Err("Ungültiger Exportname".into());
            }
            if raw.ends_with(".zip") {
                raw
            } else {
                format!("{raw}.zip")
            }
        }
        None => {
            let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            format!("reddot-diagnostics-{stamp}-{}.zip", uuid::Uuid::new_v4())
        }
    };
    Ok(exports.join(name))
}

fn write_emergency_zip(
    zip_path: &Path,
    sqlite_copy: &Path,
    manifest: &serde_json::Value,
    events_jsonl: &str,
    attachments: &[(String, String)],
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

    for (name, contents) in attachments {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(contents.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let mut writer = zip.finish().map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    writer.get_ref().sync_all().map_err(|e| e.to_string())?;
    Ok(())
}
