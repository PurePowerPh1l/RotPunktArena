//! Training session history — saved series for progress tracking.
//!
//! Single write path: [`Database::maybe_save_training_history`].
//! Policy: DB shot count is source of truth; short / empty series stay out of history.

use super::Database;
use rusqlite::{params, OptionalExtension};

/// Fixed training series length — one series = this many shots.
/// Must match `packages/domain` `TRAINING_SERIES_SHOTS`.
pub const TRAINING_SERIES_SHOTS: i64 = 10;

/// Minimum shots before a training series appears in history.
/// Equals [`TRAINING_SERIES_SHOTS`] so only complete series feed statistics.
pub const TRAINING_HISTORY_MIN_SHOTS: i64 = TRAINING_SERIES_SHOTS;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingSessionSummary {
    pub id: String,
    pub shooter_name: String,
    pub person_id: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub shot_count: i64,
    pub punkte_total: f64,
    pub teiler_sum: f64,
    pub teiler_avg: f64,
}

/// Saved training series with shot marks for the Statistik detail view.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingSessionDetail {
    pub summary: TrainingSessionSummary,
    pub shots: Vec<super::results::ResultShot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingShooterOption {
    pub person_id: Option<String>,
    pub shooter_name: String,
    pub session_count: i64,
}

/// Result of the training history auto-save decision (for UI / diagnostics).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrainingSaveInfo {
    pub saved: bool,
    pub shot_count: i64,
    pub min_shots: i64,
    /// `saved` | `empty` | `too_short` | `not_training` | `endless`
    pub reason: String,
}

impl TrainingSaveInfo {
    pub fn not_training() -> Self {
        Self {
            saved: false,
            shot_count: 0,
            min_shots: TRAINING_HISTORY_MIN_SHOTS,
            reason: "not_training".into(),
        }
    }

    pub fn ui_message(&self) -> Option<String> {
        match self.reason.as_str() {
            "saved" => Some(format!(
                "Serie gespeichert ({} Schüsse) — unter Statistik sichtbar",
                self.shot_count
            )),
            "too_short" => Some(format!(
                "Serie beendet ({} Schüsse) — Statistik nur bei voller {}er-Serie",
                self.shot_count, self.min_shots
            )),
            "empty" => Some("Serie beendet — keine Schüsse, nichts gespeichert".into()),
            "endless" => Some(format!(
                "Endlosmodus beendet ({} Schüsse) — nicht in Statistik",
                self.shot_count
            )),
            _ => None,
        }
    }
}

impl Database {
    /// Decide whether this ended training session belongs in history.
    ///
    /// Policy (DB is source of truth):
    /// - competition sessions → never history
    /// - 0 shots → not saved
    /// - 1..MIN-1 → not saved (anti-spam)
    /// - ≥ MIN → set `training_saved` once (idempotent)
    pub fn maybe_save_training_history(
        &self,
        session_id: &str,
        is_training: bool,
    ) -> Result<TrainingSaveInfo, String> {
        let min_shots = TRAINING_HISTORY_MIN_SHOTS;
        if !is_training {
            return Ok(TrainingSaveInfo::not_training());
        }

        let row: Option<(i64, Option<String>)> = self
            .conn
            .query_row(
                "SELECT training_saved, competition_id FROM sessions WHERE id = ?1",
                params![session_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let Some((already_flagged, competition_id)) = row else {
            return Ok(TrainingSaveInfo {
                saved: false,
                shot_count: 0,
                min_shots,
                reason: "empty".into(),
            });
        };
        if competition_id.is_some() {
            return Ok(TrainingSaveInfo::not_training());
        }

        let shot_count = self.count_session_shots(session_id)?;
        if already_flagged != 0 {
            return Ok(TrainingSaveInfo {
                saved: true,
                shot_count,
                min_shots,
                reason: "saved".into(),
            });
        }
        if shot_count <= 0 {
            return Ok(TrainingSaveInfo {
                saved: false,
                shot_count: 0,
                min_shots,
                reason: "empty".into(),
            });
        }
        if shot_count < min_shots {
            return Ok(TrainingSaveInfo {
                saved: false,
                shot_count,
                min_shots,
                reason: "too_short".into(),
            });
        }

        let n = self
            .conn
            .execute(
                "UPDATE sessions SET training_saved = 1
                 WHERE id = ?1 AND competition_id IS NULL AND training_saved = 0",
                params![session_id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            // Race: another writer flagged it — treat as saved.
            return Ok(TrainingSaveInfo {
                saved: true,
                shot_count,
                min_shots,
                reason: "saved".into(),
            });
        }

        Ok(TrainingSaveInfo {
            saved: true,
            shot_count,
            min_shots,
            reason: "saved".into(),
        })
    }

    pub fn list_saved_training_sessions(
        &self,
        limit: i64,
        person_id: Option<&str>,
        shooter_name: Option<&str>,
    ) -> Result<Vec<TrainingSessionSummary>, String> {
        let lim = if limit <= 0 { 80 } else { limit.min(200) };
        let person = person_id.map(str::trim).filter(|s| !s.is_empty());
        let name = shooter_name.map(str::trim).filter(|s| !s.is_empty());

        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, s.shooter_name, s.person_id, s.started_at, s.ended_at,
                        COUNT(sh.id) AS shot_count,
                        COALESCE(SUM(sh.score), 0) AS punkte_total,
                        COALESCE(SUM(CAST(sh.distance_raw AS REAL) / 10.0), 0) AS teiler_sum,
                        COALESCE(AVG(CAST(sh.distance_raw AS REAL) / 10.0), 0) AS teiler_avg
                 FROM sessions s
                 LEFT JOIN shots sh ON sh.session_id = s.id
                 WHERE s.competition_id IS NULL
                   AND s.training_saved = 1
                   AND s.ended_at IS NOT NULL
                   AND (?1 IS NULL OR s.person_id = ?1)
                   AND (
                     ?2 IS NULL
                     OR (
                       ?1 IS NOT NULL AND s.person_id = ?1
                     )
                     OR (
                       ?1 IS NULL AND s.person_id IS NULL
                       AND LOWER(TRIM(s.shooter_name)) = LOWER(TRIM(?2))
                     )
                   )
                 GROUP BY s.id
                 ORDER BY s.ended_at DESC
                 LIMIT ?3",
            )
            .map_err(|e| e.to_string())?;

        let filter_person = person;
        let filter_name = if filter_person.is_some() { None } else { name };

        let rows = stmt
            .query_map(params![filter_person, filter_name, lim], |r| {
                Ok(TrainingSessionSummary {
                    id: r.get(0)?,
                    shooter_name: r.get(1)?,
                    person_id: r.get(2)?,
                    started_at: r.get(3)?,
                    ended_at: r.get(4)?,
                    shot_count: r.get(5)?,
                    punkte_total: r.get(6)?,
                    teiler_sum: r.get(7)?,
                    teiler_avg: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        out.reverse();
        Ok(out)
    }

    /// One saved training series with shot marks (Statistik detail).
    pub fn get_training_session_detail(
        &self,
        session_id: &str,
    ) -> Result<Option<TrainingSessionDetail>, String> {
        let id = session_id.trim();
        if id.is_empty() {
            return Ok(None);
        }

        let row = self
            .conn
            .query_row(
                "SELECT s.id, s.shooter_name, s.person_id, s.started_at, s.ended_at,
                        COUNT(sh.id) AS shot_count,
                        COALESCE(SUM(sh.score), 0) AS punkte_total,
                        COALESCE(SUM(CAST(sh.distance_raw AS REAL) / 10.0), 0) AS teiler_sum,
                        COALESCE(AVG(CAST(sh.distance_raw AS REAL) / 10.0), 0) AS teiler_avg
                 FROM sessions s
                 LEFT JOIN shots sh ON sh.session_id = s.id
                 WHERE s.id = ?1
                   AND s.competition_id IS NULL
                   AND s.training_saved = 1
                   AND s.ended_at IS NOT NULL
                 GROUP BY s.id",
                params![id],
                |r| {
                    Ok(TrainingSessionSummary {
                        id: r.get(0)?,
                        shooter_name: r.get(1)?,
                        person_id: r.get(2)?,
                        started_at: r.get(3)?,
                        ended_at: r.get(4)?,
                        shot_count: r.get(5)?,
                        punkte_total: r.get(6)?,
                        teiler_sum: r.get(7)?,
                        teiler_avg: r.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let Some(summary) = row else {
            return Ok(None);
        };
        let shots = self.list_session_ui_shots(&summary.id)?;
        Ok(Some(TrainingSessionDetail { summary, shots }))
    }

    /// Distinct shooters that appear in saved training history (for filter UI).
    pub fn list_training_shooters(&self) -> Result<Vec<TrainingShooterOption>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.person_id, s.shooter_name, COUNT(*) AS session_count
                 FROM sessions s
                 WHERE s.competition_id IS NULL
                   AND s.training_saved = 1
                   AND s.ended_at IS NOT NULL
                 GROUP BY
                   CASE WHEN s.person_id IS NOT NULL THEN s.person_id ELSE '' END,
                   CASE WHEN s.person_id IS NOT NULL THEN '' ELSE LOWER(TRIM(s.shooter_name)) END
                 ORDER BY MAX(s.ended_at) DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TrainingShooterOption {
                    person_id: r.get(0)?,
                    shooter_name: r.get(1)?,
                    session_count: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// Hide saved training sessions from history (soft clear — shots stay in DB).
    /// Returns how many sessions were unmarked.
    pub fn clear_training_history(
        &self,
        person_id: Option<&str>,
        shooter_name: Option<&str>,
    ) -> Result<i64, String> {
        let person = person_id.map(str::trim).filter(|s| !s.is_empty());
        let name = shooter_name.map(str::trim).filter(|s| !s.is_empty());
        let filter_person = person;
        let filter_name = if filter_person.is_some() { None } else { name };

        let n = self
            .conn
            .execute(
                "UPDATE sessions
                 SET training_saved = 0
                 WHERE competition_id IS NULL
                   AND training_saved = 1
                   AND ended_at IS NOT NULL
                   AND (?1 IS NULL OR person_id = ?1)
                   AND (
                     ?2 IS NULL
                     OR (
                       ?1 IS NOT NULL AND person_id = ?1
                     )
                     OR (
                       ?1 IS NULL AND person_id IS NULL
                       AND LOWER(TRIM(shooter_name)) = LOWER(TRIM(?2))
                     )
                   )",
                params![filter_person, filter_name],
            )
            .map_err(|e| e.to_string())?;
        Ok(n as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::protocol::build_synthetic_shot_frame;

    fn seed_training(db: &mut Database, shots: i64) -> String {
        let session = db
            .start_session("Test", None, None, None)
            .expect("session");
        for i in 0..shots {
            let x = format!("{:05}", i + 1);
            let frame = build_synthetic_shot_frame("10.5", "012.30", &x, "00040").unwrap();
            let _ = db
                .ingest_raw_frame(&session.id, &frame, "test", None)
                .expect("ingest");
        }
        session.id
    }

    #[test]
    fn skips_empty_and_short_series() {
        let mut db = Database::open_in_memory().unwrap();
        let empty = seed_training(&mut db, 0);
        let short = seed_training(&mut db, 3);
        let ok = seed_training(&mut db, TRAINING_HISTORY_MIN_SHOTS);

        let a = db.maybe_save_training_history(&empty, true).unwrap();
        assert_eq!(a.reason, "empty");
        assert!(!a.saved);

        let b = db.maybe_save_training_history(&short, true).unwrap();
        assert_eq!(b.reason, "too_short");
        assert_eq!(b.shot_count, 3);
        assert!(!b.saved);

        let c = db.maybe_save_training_history(&ok, true).unwrap();
        assert_eq!(c.reason, "saved");
        assert!(c.saved);
        assert_eq!(c.shot_count, TRAINING_HISTORY_MIN_SHOTS);

        let again = db.maybe_save_training_history(&ok, true).unwrap();
        assert!(again.saved);
    }

    #[test]
    fn ignores_competition_sessions() {
        let db = Database::open_in_memory().unwrap();
        let info = db.maybe_save_training_history("any", false).unwrap();
        assert_eq!(info, TrainingSaveInfo::not_training());
    }
}
