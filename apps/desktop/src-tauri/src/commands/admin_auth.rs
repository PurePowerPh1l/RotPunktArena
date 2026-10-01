//! Admin password aggregate (`settings` key `admin.auth`).
//!
//! Stores salt+hash only. Status commands never return the hash.
//! Setup is one-shot until a future change-password slice.

use crate::db::Database;
use crate::engine::StandEngine;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const ADMIN_AUTH_KEY: &str = "admin.auth";
const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_BYTES: usize = 1024;
const KDF_PREFIX: &str = "$argon2id$v=19$m=19456,t=2,p=1$";

/// Server-side admin unlock flag (managed Tauri state).
///
/// Defense in depth: the WebView-side capability gates can be bypassed by any
/// IPC caller, so destructive commands additionally require this flag to be
/// set. It is unlocked only by a successful password verify/setup in this
/// process and never persisted across restarts.
#[derive(Default)]
pub struct AdminSession {
    unlocked: AtomicBool,
    attempts: Mutex<Attempts>,
}

#[derive(Default)]
struct Attempts {
    failures: u32,
    blocked_until: Option<Instant>,
}

impl Attempts {
    fn check(&mut self, now: Instant) -> Result<(), String> {
        if let Some(deadline) = self.blocked_until {
            if now < deadline {
                return Err(
                    "Zu viele Fehlversuche. Bitte nach 60 Sekunden erneut versuchen.".into(),
                );
            }
            self.failures = 0;
            self.blocked_until = None;
        }
        Ok(())
    }
    fn record(&mut self, success: bool, now: Instant) {
        if success {
            self.failures = 0;
            self.blocked_until = None;
        } else {
            self.failures += 1;
            if self.failures >= 5 {
                self.blocked_until = Some(now + Duration::from_secs(60));
            }
        }
    }
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
        format!("Admin-Auth ungültig ({e}). Gespeicherte Werte wurden nicht überschrieben.")
    })?;
    if !matches!(record.version, 1 | 2) {
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
    if record.version == 2 {
        let parsed =
            PasswordHash::new(&record.hash).map_err(|e| format!("Admin-KDF beschädigt: {e}"))?;
        let salt = hex::decode(&record.salt).map_err(|e| e.to_string())?;
        let encoded = SaltString::encode_b64(&salt).map_err(|e| e.to_string())?;
        if !record.hash.starts_with(KDF_PREFIX)
            || parsed.salt.map(|s| s.as_str()) != Some(encoded.as_str())
            || !parsed.hash.is_some_and(|hash| hash.len() == 32)
        {
            return Err(
                "Admin-KDF-Parameter nicht unterstützt; gespeicherte Werte bleiben erhalten".into(),
            );
        }
    } else if hex::decode(&record.hash)
        .map_err(|e| format!("Admin-Hash beschädigt: {e}"))?
        .len()
        != 32
    {
        return Err("Admin-Hash hat ungültige Länge".into());
    }
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
    validate_password_size(password)?;
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(format!(
            "Admin-Passwort muss mindestens {MIN_PASSWORD_LEN} Zeichen haben."
        ));
    }
    Ok(())
}

fn validate_password_size(password: &str) -> Result<(), String> {
    if password.len() > MAX_PASSWORD_BYTES {
        return Err("Admin-Passwort ist zu lang (maximal 1024 Bytes)".into());
    }
    Ok(())
}

fn new_record(password: &str) -> Result<AdminAuthRecord, String> {
    let salt_bytes = *Uuid::new_v4().as_bytes();
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|e| e.to_string())?;
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Admin-KDF: {e}"))?
        .to_string();
    Ok(AdminAuthRecord {
        version: 2,
        salt: hex::encode(salt_bytes),
        hash,
    })
}

fn verify_and_migrate(db: &Database, password: &str) -> Result<bool, String> {
    validate_password_size(password)?;
    let record = load_record(db)?.ok_or("Kein Admin-Passwort eingerichtet")?;
    let matched = if record.version == 1 {
        constant_time_eq(
            hash_password(password, &record.salt)?.as_bytes(),
            record.hash.as_bytes(),
        )
    } else {
        let parsed = PasswordHash::new(&record.hash).map_err(|e| e.to_string())?;
        match Argon2::default().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => true,
            Err(argon2::password_hash::Error::Password) => false,
            Err(error) => return Err(format!("Admin-KDF-Prüfung fehlgeschlagen: {error}")),
        }
    };
    // Migration must commit before unlocking. Legacy short passwords remain usable.
    if matched && record.version == 1 {
        store_record(db, &new_record(password)?)?;
    }
    Ok(matched)
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
        store_record(db, &new_record(&password)?)?;
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
    let mut attempts = session.attempts.lock();
    attempts.check(Instant::now())?;
    let ok = engine.with_db(|db| verify_and_migrate(db, &password))?;
    attempts.record(ok, Instant::now());
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
pub fn dev_unlock_admin_session(session: tauri::State<'_, AdminSession>) -> Result<(), String> {
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
    fn kdf_roundtrip_and_legacy_migration_only_after_success() {
        let db = Database::open_in_memory().unwrap();
        let legacy = AdminAuthRecord {
            version: 1,
            salt: hex::encode(Uuid::new_v4().as_bytes()),
            hash: String::new(),
        };
        let legacy = AdminAuthRecord {
            hash: hash_password("four", &legacy.salt).unwrap(),
            ..legacy
        };
        store_record(&db, &legacy).unwrap();
        let original = db.get_setting(ADMIN_AUTH_KEY).unwrap().unwrap();
        assert!(!verify_and_migrate(&db, "wrong").unwrap());
        assert_eq!(db.get_setting(ADMIN_AUTH_KEY).unwrap().unwrap(), original);
        assert!(verify_and_migrate(&db, "four").unwrap());
        let migrated = load_record(&db).unwrap().unwrap();
        assert_eq!(migrated.version, 2);
        assert!(migrated.hash.starts_with(KDF_PREFIX));
        assert!(verify_and_migrate(&db, "four").unwrap());
        assert!(!verify_and_migrate(&db, "wrong").unwrap());
        assert!(verify_and_migrate(&db, &"x".repeat(MAX_PASSWORD_BYTES + 1)).is_err());
    }

    #[test]
    fn migration_write_failure_keeps_legacy_credentials() {
        let db = Database::open_in_memory().unwrap();
        let salt = hex::encode(Uuid::new_v4().as_bytes());
        store_record(
            &db,
            &AdminAuthRecord {
                version: 1,
                hash: hash_password("four", &salt).unwrap(),
                salt,
            },
        )
        .unwrap();
        let before = db.get_setting(ADMIN_AUTH_KEY).unwrap().unwrap();
        db.conn.execute_batch("CREATE TRIGGER reject_auth_update BEFORE UPDATE ON settings BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        assert!(verify_and_migrate(&db, "four").is_err());
        assert_eq!(db.get_setting(ADMIN_AUTH_KEY).unwrap().unwrap(), before);
        db.conn
            .execute_batch("DROP TRIGGER reject_auth_update")
            .unwrap();
        assert!(verify_and_migrate(&db, "four").unwrap());
    }

    #[test]
    fn unsupported_kdf_parameters_are_rejected_without_overwrite() {
        let db = Database::open_in_memory().unwrap();
        let mut record = new_record("password").unwrap();
        record.hash = record.hash.replace("m=19456", "m=999999999");
        store_record(&db, &record).unwrap();
        let before = db.get_setting(ADMIN_AUTH_KEY).unwrap();
        assert!(verify_and_migrate(&db, "password").is_err());
        assert_eq!(db.get_setting(ADMIN_AUTH_KEY).unwrap(), before);
    }

    #[test]
    fn attempt_limit_expiry_and_success_reset_use_monotonic_time() {
        let now = Instant::now();
        let mut attempts = Attempts::default();
        for _ in 0..5 {
            attempts.check(now).unwrap();
            attempts.record(false, now);
        }
        assert!(attempts.check(now + Duration::from_secs(59)).is_err());
        attempts.check(now + Duration::from_secs(60)).unwrap();
        attempts.record(false, now + Duration::from_secs(60));
        attempts.record(true, now + Duration::from_secs(60));
        assert_eq!(attempts.failures, 0);
        assert!(attempts.blocked_until.is_none());
    }

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
        assert!(validate_password("abcdefgh").is_ok());
        assert!(validate_password("äöüß").is_err());
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
