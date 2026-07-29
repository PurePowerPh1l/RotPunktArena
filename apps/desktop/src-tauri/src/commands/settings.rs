//! UI preferences aggregate (`settings` key `ui.prefs`).
//!
//! One validated JSON blob; single write path via `Db::set_setting`.
//! Unknown JSON fields are tolerated on read (forward compat) and dropped on
//! the next full write. Invalid JSON / types / enum values fail loudly.

use crate::db::Database;
use crate::engine::StandEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const UI_PREFS_KEY: &str = "ui.prefs";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppViewPref {
    Live,
    History,
    Bureau,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorSchemePref {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScoreDisplayPref {
    Punkte,
    Teiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HitFeedbackPref {
    Normal,
    Reduced,
    Minimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetFitPref {
    Auto,
    Calm,
    Aggressive,
}

/// Full UI prefs DTO — sole aggregate for Settings Allgemein / Darstellung / Arena.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPrefs {
    pub start_view: AppViewPref,
    pub remember_last_view: bool,
    pub last_view: Option<AppViewPref>,
    pub compact_ui: bool,
    pub large_text: bool,
    /// Bigger type + controls for distance / hall readability.
    /// `serde(default)` so older stored prefs without the field still load.
    #[serde(default)]
    pub extra_large_ui: bool,
    pub color_scheme: ColorSchemePref,
    pub reduced_motion: bool,
    pub score_display: ScoreDisplayPref,
    pub remember_score_display: bool,
    pub hit_feedback: HitFeedbackPref,
    pub target_fit: TargetFitPref,
}

impl Default for UiPrefs {
    /// Sole authoritative default source for this aggregate.
    fn default() -> Self {
        Self {
            start_view: AppViewPref::Live,
            remember_last_view: false,
            last_view: None,
            compact_ui: false,
            large_text: false,
            extra_large_ui: false,
            color_scheme: ColorSchemePref::System,
            reduced_motion: false,
            score_display: ScoreDisplayPref::Punkte,
            remember_score_display: false,
            hit_feedback: HitFeedbackPref::Normal,
            target_fit: TargetFitPref::Auto,
        }
    }
}

fn parse_ui_prefs_json(raw: &str) -> Result<UiPrefs, String> {
    serde_json::from_str::<UiPrefs>(raw).map_err(|e| {
        format!(
            "UI-Einstellungen ungültig ({e}). Bitte Einstellungen zurücksetzen oder erneut speichern — die gespeicherten Werte wurden nicht überschrieben."
        )
    })
}

fn load_ui_prefs(db: &Database) -> Result<UiPrefs, String> {
    match db.get_setting(UI_PREFS_KEY)? {
        None => Ok(UiPrefs::default()),
        Some(raw) => parse_ui_prefs_json(&raw),
    }
}

fn store_ui_prefs(db: &Database, prefs: &UiPrefs) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    db.set_setting(UI_PREFS_KEY, &json)
}

/// Missing key → defaults. Corrupt / invalid stored JSON → error (no silent overwrite).
#[tauri::command]
pub fn get_ui_prefs(engine: tauri::State<'_, Arc<StandEngine>>) -> Result<UiPrefs, String> {
    engine.with_db(load_ui_prefs)
}

/// Full validated DTO write — exactly one `Db::set_setting("ui.prefs", …)`.
#[tauri::command]
pub fn set_ui_prefs(
    engine: tauri::State<'_, Arc<StandEngine>>,
    prefs: UiPrefs,
) -> Result<UiPrefs, String> {
    // `prefs` already deserialized by Tauri/serde (rejects unknown enums / bad types).
    engine.with_db(|db| {
        store_ui_prefs(db, &prefs)?;
        Ok(prefs.clone())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_key_returns_exact_defaults() {
        let db = Database::open_in_memory().unwrap();
        let prefs = load_ui_prefs(&db).unwrap();
        assert_eq!(prefs, UiPrefs::default());
    }

    #[test]
    fn roundtrip_preserves_full_dto() {
        let db = Database::open_in_memory().unwrap();
        let mut prefs = UiPrefs::default();
        prefs.start_view = AppViewPref::History;
        prefs.remember_last_view = true;
        prefs.last_view = Some(AppViewPref::Bureau);
        prefs.compact_ui = true;
        prefs.large_text = true;
        prefs.extra_large_ui = true;
        prefs.color_scheme = ColorSchemePref::Dark;
        prefs.reduced_motion = true;
        prefs.score_display = ScoreDisplayPref::Teiler;
        prefs.remember_score_display = true;
        prefs.hit_feedback = HitFeedbackPref::Minimal;
        prefs.target_fit = TargetFitPref::Calm;

        store_ui_prefs(&db, &prefs).unwrap();
        let loaded = load_ui_prefs(&db).unwrap();
        assert_eq!(loaded, prefs);
    }

    #[test]
    fn corrupt_json_returns_stable_error_without_overwrite() {
        let db = Database::open_in_memory().unwrap();
        db.set_setting(UI_PREFS_KEY, "{not-json").unwrap();
        let err = load_ui_prefs(&db).unwrap_err();
        assert!(
            err.contains("UI-Einstellungen ungültig"),
            "unexpected error: {err}"
        );
        assert!(
            err.contains("nicht überschrieben"),
            "missing recovery hint: {err}"
        );
        // Stored blob must remain untouched.
        let raw = db.get_setting(UI_PREFS_KEY).unwrap().unwrap();
        assert_eq!(raw, "{not-json");
    }

    #[test]
    fn wrong_field_type_rejected() {
        let db = Database::open_in_memory().unwrap();
        let bad = json!({
            "startView": "live",
            "rememberLastView": "yes",
            "lastView": null,
            "compactUi": false,
            "largeText": false,
            "extraLargeUi": false,
            "colorScheme": "system",
            "reducedMotion": false,
            "scoreDisplay": "punkte",
            "rememberScoreDisplay": false,
            "hitFeedback": "normal",
            "targetFit": "auto"
        })
        .to_string();
        db.set_setting(UI_PREFS_KEY, &bad).unwrap();
        let err = load_ui_prefs(&db).unwrap_err();
        assert!(err.contains("UI-Einstellungen ungültig"), "{err}");
    }

    #[test]
    fn unknown_enum_value_rejected() {
        let db = Database::open_in_memory().unwrap();
        let bad = json!({
            "startView": "live",
            "rememberLastView": false,
            "lastView": null,
            "compactUi": false,
            "largeText": false,
            "extraLargeUi": false,
            "colorScheme": "neon",
            "reducedMotion": false,
            "scoreDisplay": "punkte",
            "rememberScoreDisplay": false,
            "hitFeedback": "normal",
            "targetFit": "auto"
        })
        .to_string();
        db.set_setting(UI_PREFS_KEY, &bad).unwrap();
        let err = load_ui_prefs(&db).unwrap_err();
        assert!(err.contains("UI-Einstellungen ungültig"), "{err}");
    }

    #[test]
    fn forward_compat_unknown_field_tolerated_then_stripped_on_write() {
        let db = Database::open_in_memory().unwrap();
        let mut base = serde_json::to_value(UiPrefs::default()).unwrap();
        let obj = base.as_object_mut().unwrap();
        obj.insert("futurePreference".into(), json!(true));
        let with_future = base.to_string();
        db.set_setting(UI_PREFS_KEY, &with_future).unwrap();

        let loaded = load_ui_prefs(&db).unwrap();
        assert_eq!(loaded, UiPrefs::default());

        store_ui_prefs(&db, &loaded).unwrap();
        let stored = db.get_setting(UI_PREFS_KEY).unwrap().unwrap();
        assert!(
            !stored.contains("futurePreference"),
            "unknown field should be dropped on full write: {stored}"
        );
        let again = load_ui_prefs(&db).unwrap();
        assert_eq!(again, UiPrefs::default());
    }

    #[test]
    fn default_json_shape_is_stable_camel_case() {
        let json = serde_json::to_value(UiPrefs::default()).unwrap();
        let obj = json.as_object().unwrap();
        for key in [
            "startView",
            "rememberLastView",
            "lastView",
            "compactUi",
            "largeText",
            "extraLargeUi",
            "colorScheme",
            "reducedMotion",
            "scoreDisplay",
            "rememberScoreDisplay",
            "hitFeedback",
            "targetFit",
        ] {
            assert!(obj.contains_key(key), "missing key {key}");
        }
        assert_eq!(obj["startView"], "live");
        assert_eq!(obj["scoreDisplay"], "punkte");
        assert_eq!(obj["colorScheme"], "system");
        assert_eq!(obj["lastView"], serde_json::Value::Null);
    }

    #[test]
    fn legacy_prefs_without_extra_large_ui_still_load() {
        let db = Database::open_in_memory().unwrap();
        let legacy = json!({
            "startView": "live",
            "rememberLastView": false,
            "lastView": null,
            "compactUi": false,
            "largeText": true,
            "colorScheme": "dark",
            "reducedMotion": false,
            "scoreDisplay": "punkte",
            "rememberScoreDisplay": false,
            "hitFeedback": "normal",
            "targetFit": "auto"
        })
        .to_string();
        db.set_setting(UI_PREFS_KEY, &legacy).unwrap();
        let prefs = load_ui_prefs(&db).unwrap();
        assert!(prefs.large_text);
        assert!(!prefs.extra_large_ui);
    }
}
