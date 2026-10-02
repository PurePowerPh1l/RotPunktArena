//! Admin: DB backup / restore / wipe.
//!
//! Restore and wipe are irreversible, so they require the server-side
//! [`AdminSession`] unlock in addition to the UI capability gate.

use crate::commands::AdminSession;
use crate::engine::StandEngine;
use chrono::Local;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn prepare_app_update(engine: tauri::State<'_, Arc<StandEngine>>) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.prepare_update())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn finish_app_update(engine: tauri::State<'_, Arc<StandEngine>>) {
    engine.finish_update();
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupHealth {
    snapshot: crate::db::SnapshotHealth,
    storage_bytes: u64,
}

#[tauri::command]
pub async fn get_backup_health(app: AppHandle) -> Result<BackupHealth, String> {
    let directory = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut health = crate::db::snapshot_health();
        if health.last_completed_at.is_none() {
            health.last_completed_at = directory
                .join("snapshots/latest.sqlite")
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
        }
        let mut bytes = 0;
        for directory in [
            directory.clone(),
            directory.join("snapshots"),
            directory.join("backups"),
            directory.join("exports"),
        ] {
            if !directory.exists() {
                continue;
            }
            for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
                let metadata = entry
                    .map_err(|e| e.to_string())?
                    .metadata()
                    .map_err(|e| e.to_string())?;
                if metadata.is_file() {
                    bytes += metadata.len();
                }
            }
        }
        Ok(BackupHealth {
            snapshot: health,
            storage_bytes: bytes,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn retry_snapshot(engine: tauri::State<'_, Arc<StandEngine>>) -> Result<(), String> {
    engine.with_db(|db| {
        let id: String = db
            .conn
            .query_row(
                "SELECT id FROM sessions ORDER BY started_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| "Noch keine Session zum Sichern".to_string())?;
        db.spawn_session_boundary_snapshot(&id);
        Ok(())
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbBackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

fn backups_dir<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
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
    let name = format!("reddot-{stamp}-{}.sqlite", uuid::Uuid::new_v4());
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
        let modified_at = meta.modified().ok().map(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.to_rfc3339()
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
pub fn restore_db_backup<R: tauri::Runtime>(
    app: AppHandle<R>,
    engine: tauri::State<'_, Arc<StandEngine>>,
    session: tauri::State<'_, AdminSession>,
    name: String,
) -> Result<String, String> {
    session.require()?;
    let name = name.trim();
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return Err("Ungültiger Backup-Name".into());
    }
    let src = backups_dir(&app)?.join(name);
    if !src.is_file() {
        return Err("Backup nicht gefunden".into());
    }
    engine.swap_database_file(&src)?;
    session.lock();
    Ok(src.to_string_lossy().into_owned())
}

/// Wipe all app data by replacing the DB with a fresh migrated file.
/// Requires the server-side admin unlock in addition to the UI gate.
#[tauri::command]
pub fn reset_all_database(
    engine: tauri::State<'_, Arc<StandEngine>>,
    session: tauri::State<'_, AdminSession>,
) -> Result<(), String> {
    session.require()?;
    engine.reset_database_to_empty()?;
    session.lock();
    Ok(())
}
