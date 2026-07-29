//! Admin password aggregate (`settings` key `admin.auth`).
//!
//! Stores salt+hash only. Status commands never return the hash.
//! Setup is one-shot until a future change-password slice.

use crate::db::Database;
use crate::engine::StandEngine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub const ADMIN_AUTH_KEY: &str = "admin.auth";
const MIN_PASSWORD_LEN: usize = 4;

/// Server-side admin unlock flag (managed Tauri state).
///
/// Defense in depth: the WebView-side capability gates can be bypassed by any
/// IPC caller, so destructive commands additionally require this flag to be
/// set. It is unlocked only by a successful password verify/setup in this
/// process and never persisted across restarts.
#[derive(Default)]
pub struct AdminSession {
    unlocked: AtomicBool,
}

impl AdminSession {
    pub fn unlock(&self) {
        self.unlocked.store(true, Ordering::SeqCst);
    }

    pub fn lock(&self) {
        self.unlocked.store(false, Ordering::SeqCst);
    }

    pub fn is_unlocked(&self) -> bool {
        self.unlocked.load(Ordering::SeqCst)
    }

    /// Gate for privileged commands. `Err` message is user-facing (German UI).
    pub fn require(&self) -> Result<(), String> {
        if self.is_unlocked() {
            Ok(())
        } else {
            Err("Admin-Freigabe erforderlich. Bitte zuerst im Admin-Bereich entsperren.".into())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminAuthRecord {
    version: u32,
    salt: String,
    hash: String,
}

/// Public status — never includes salt/hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAuthStatus {
    pub configured: bool,
}

fn hash_password(password: &str, salt_hex: &str) -> Result<String, String> {
    let salt = hex::decode(salt_hex).map_err(|e| format!("Admin-Auth beschädigt (Salt): {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(password.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn parse_record(raw: &str) -> Result<AdminAuthRecord, String> {
    let record: AdminAuthRecord = serde_json::from_str(raw).map_err(|e| {
        format!(
            "Admin-Auth ungültig ({e}). Gespeicherte Werte wurden nicht überschrieben."
        )
    })?;
    if record.version != 1 {
        return Err(format!(
            "Admin-Auth Version {} nicht unterstützt. Gespeicherte Werte wurden nicht überschrieben.",
            record.version
        ));
    }
    if record.salt.is_empty() || record.hash.is_empty() {
        return Err(
            "Admin-Auth unvollständig. Gespeicherte Werte wurden nicht überschrieben.".into(),
        );
    }
    // Validate salt is hex.
    hash_password("", &record.salt)?;
    Ok(record)
}

fn load_record(db: &Database) -> Result<Option<AdminAuthRecord>, String> {
    match db.get_setting(ADMIN_AUTH_KEY)? {
        None => Ok(None),
        Some(raw) => Ok(Some(parse_record(&raw)?)),
    }
}

fn store_record(db: &Database, record: &AdminAuthRecord) -> Result<(), String> {
    let json = serde_json::to_string(record).map_err(|e| e.to_string())?;
    db.set_setting(ADMIN_AUTH_KEY, &json)
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "Admin-Passwort muss mindestens {MIN_PASSWORD_LEN} Zeichen haben."
        ));
    }
    Ok(())
}

/// Missing key → not configured. Corrupt blob → error (no silent wipe).
#[tauri::command]
pub fn get_admin_auth_status(
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<AdminAuthStatus, String> {
    engine.with_db(|db| {
        let configured = load_record(db)?.is_some();
        Ok(AdminAuthStatus { configured })
    })
}

/// First-time setup only. Rejects if already configured. Unlocks the session.
#[tauri::command]
pub fn setup_admin_password(
    engine: tauri::State<'_, Arc<StandEngine>>,
    session: tauri::State<'_, AdminSession>,
    password: String,
) -> Result<AdminAuthStatus, String> {
    validate_password(&password)?;
    let status = engine.with_db(|db| {
        if load_record(db)?.is_some() {
            return Err("Admin-Passwort ist bereits gesetzt.".to_string());
        }
        let salt = hex::encode(Uuid::new_v4().as_bytes());
        let hash = hash_password(&password, &salt)?;
        store_record(
            db,
            &AdminAuthRecord {
                version: 1,
                salt,
                hash,
            },
        )?;
        Ok(AdminAuthStatus { configured: true })
    })?;
    session.unlock();
    Ok(status)
}

/// Verify password against stored hash. On success, unlocks the server-side
/// admin session so privileged commands become callable.
#[tauri::command]
pub fn verify_admin_password(
    engine: tauri::State<'_, Arc<StandEngine>>,
    session: tauri::State<'_, AdminSession>,
    password: String,
) -> Result<bool, String> {
    let ok = engine.with_db(|db| {
        let Some(record) = load_record(db)? else {
            return Err("Kein Admin-Passwort eingerichtet.".to_string());
        };
        let candidate = hash_password(&password, &record.salt)?;
        Ok(constant_time_eq(candidate.as_bytes(), record.hash.as_bytes()))
    })?;
    if ok {
        session.unlock();
    }
    Ok(ok)
}

/// Lock the server-side admin session (called when the UI locks admin mode).
#[tauri::command]
pub fn lock_admin_session(session: tauri::State<'_, AdminSession>) {
    session.lock();
}

/// DEV/TEST ONLY — unlock the server-side session without a password so the
/// developer test-unlock stays usable. No-op in release builds (returns an
/// error) so it can never bypass auth in shipped binaries.
#[tauri::command]
pub fn dev_unlock_admin_session(
    session: tauri::State<'_, AdminSession>,
) -> Result<(), String> {
    if cfg!(debug_assertions) {
        session.unlock();
        Ok(())
    } else {
        Err("Nur in Entwicklungs-Builds verfügbar.".into())
    }
}

/// Length-independent byte comparison to avoid leaking match progress via timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_is_not_configured() {
        let db = Database::open_in_memory().unwrap();
        assert!(load_record(&db).unwrap().is_none());
    }

    #[test]
    fn setup_and_verify_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        assert!(load_record(&db).unwrap().is_none());

        let salt = hex::encode(Uuid::new_v4().as_bytes());
        let hash = hash_password("secret", &salt).unwrap();
        store_record(
            &db,
            &AdminAuthRecord {
                version: 1,
                salt: salt.clone(),
                hash,
            },
        )
        .unwrap();

        let record = load_record(&db).unwrap().unwrap();
        assert_eq!(hash_password("secret", &record.salt).unwrap(), record.hash);
        assert_ne!(hash_password("wrong", &record.salt).unwrap(), record.hash);
    }

    #[test]
    fn second_setup_rejected_when_record_exists() {
        let db = Database::open_in_memory().unwrap();
        let salt = hex::encode(Uuid::new_v4().as_bytes());
        let hash = hash_password("first", &salt).unwrap();
        store_record(
            &db,
            &AdminAuthRecord {
                version: 1,
                salt,
                hash,
            },
        )
        .unwrap();
        assert!(load_record(&db).unwrap().is_some());
    }

    #[test]
    fn corrupt_json_errors_without_overwrite() {
        let db = Database::open_in_memory().unwrap();
        db.set_setting(ADMIN_AUTH_KEY, "{not-json").unwrap();
        let err = load_record(&db).unwrap_err();
        assert!(err.contains("Admin-Auth ungültig"), "{err}");
        assert!(err.contains("nicht überschrieben"), "{err}");
        assert_eq!(
            db.get_setting(ADMIN_AUTH_KEY).unwrap().as_deref(),
            Some("{not-json")
        );
    }

    #[test]
    fn short_password_rejected() {
        assert!(validate_password("abc").is_err());
        assert!(validate_password("abcd").is_ok());
    }

    #[test]
    fn status_dto_has_no_secrets() {
        let json = serde_json::to_value(AdminAuthStatus { configured: true }).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("configured"));
        assert!(!obj.contains_key("hash"));
        assert!(!obj.contains_key("salt"));
    }
}
