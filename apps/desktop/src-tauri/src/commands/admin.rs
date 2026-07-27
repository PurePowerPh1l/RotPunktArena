//! Admin: DB backup / restore / wipe — gated in the UI (admin + developer mode).

use crate::engine::StandEngine;
use chrono::Local;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbBackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

fn backups_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = data_dir.join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Consistent backup via VACUUM INTO under `app_data/backups/`.
#[tauri::command]
pub fn create_db_backup(
    app: AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<DbBackupInfo, String> {
    let dir = backups_dir(&app)?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let name = format!("reddot-{stamp}.sqlite");
    let dest = dir.join(&name);
    engine.with_db(|db| db.vacuum_into(&dest))?;
    let meta = std::fs::metadata(&dest).map_err(|e| e.to_string())?;
    Ok(DbBackupInfo {
        name,
        path: dest.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        modified_at: Some(Local::now().to_rfc3339()),
    })
}

#[tauri::command]
pub fn list_db_backups(app: AppHandle) -> Result<Vec<DbBackupInfo>, String> {
    let dir = backups_dir(&app)?;
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for ent in entries {
        let ent = ent.map_err(|e| e.to_string())?;
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("sqlite") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("backup.sqlite")
            .to_string();
        let meta = ent.metadata().map_err(|e| e.to_string())?;
        let modified_at = meta.modified().ok().and_then(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            Some(dt.to_rfc3339())
        });
        out.push(DbBackupInfo {
            name,
            path: path.to_string_lossy().into_owned(),
            size_bytes: meta.len(),
            modified_at,
        });
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// Replace the live DB file with a backup (session must be stopped).
#[tauri::command]
pub fn restore_db_backup(
    app: AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    name: String,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return Err("Ungültiger Backup-Name".into());
    }
    let src = backups_dir(&app)?.join(name);
    if !src.is_file() {
        return Err("Backup nicht gefunden".into());
    }
    engine.swap_database_file(&src)?;
    Ok(src.to_string_lossy().into_owned())
}

/// Wipe all app data by replacing the DB with a fresh migrated file.
/// UI must gate this behind admin + developer mode.
#[tauri::command]
pub fn reset_all_database(
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<(), String> {
    engine.reset_database_to_empty()
}
