use super::{competition_kind, competition_status, entry_status, Database};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Competition {
    pub id: String,
    pub name: String,
    pub date: String,
    pub discipline: String,
    pub max_shots: i64,
    pub scoring_mode: String,
    pub status: String,
    pub created_at: String,
    /// `competition` (default) or `training` (Trainingswettkampf with start list).
    #[serde(default = "default_competition_kind")]
    pub kind: String,
    #[serde(default)]
    pub nachkauf_enabled: bool,
    /// Legacy DB column (extra-shot cap). Create stores 0; Nachkauf = full series restarts.
    #[serde(default)]
    pub nachkauf_shots: i64,
    #[serde(default)]
    pub team_scoring_enabled: bool,
    /// How many best shooters count toward the team total (default 3).
    #[serde(default = "default_team_count")]
    pub team_count: i64,
    /// When true, points are tenths (10.5); when false, whole rings (`floor`).
    /// New competitions default false; existing DBs backfilled to true.
    #[serde(default)]
    pub tenths_enabled: bool,
    /// Probeschüsse allowed at series start (unscored, ended manually).
    #[serde(default)]
    pub probe_enabled: bool,
}

fn default_team_count() -> i64 {
    3
}

fn default_competition_kind() -> String {
    competition_kind::COMPETITION.to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionEntry {
    pub id: String,
    pub competition_id: String,
    pub person_id: String,
    pub start_order: i64,
    pub status: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub club: Option<String>,
    /// Count of started Nachkauf series (full restarts after `done`), not extra shots.
    #[serde(default)]
    pub nachkauf_purchased: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompetition {
    pub name: String,
    pub date: String,
    pub discipline: String,
    pub max_shots: i64,
    pub scoring_mode: String,
    #[serde(default)]
    pub nachkauf_enabled: bool,
    /// Legacy; Create always persists 0. Nachkauf is full series restarts, not extra shots.
    #[serde(default)]
    pub nachkauf_shots: i64,
    #[serde(default)]
    pub team_scoring_enabled: bool,
    #[serde(default = "default_team_count")]
    pub team_count: i64,
    #[serde(default = "default_competition_kind")]
    pub kind: String,
    /// Default false = whole rings for new competitions.
    #[serde(default)]
    pub tenths_enabled: bool,
    #[serde(default)]
    pub probe_enabled: bool,
}

fn normalize_competition_kind(raw: &str) -> &'static str {
    match raw.trim() {
        competition_kind::TRAINING => competition_kind::TRAINING,
        _ => competition_kind::COMPETITION,
    }
}

impl Database {
    pub fn list_competitions(&self, include_archived: bool) -> Result<Vec<Competition>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, date, discipline, max_shots, scoring_mode, status, created_at,
                        COALESCE(nachkauf_enabled, 0), COALESCE(nachkauf_shots, 0),
                        COALESCE(team_scoring_enabled, 0), COALESCE(team_count, 3),
                        COALESCE(kind, 'competition'), COALESCE(tenths_enabled, 1),
                        COALESCE(probe_enabled, 0)
                 FROM competitions
                 WHERE (?1 = 1 OR status != ?2)
                 ORDER BY date DESC, created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                params![
                    if include_archived { 1i64 } else { 0i64 },
                    competition_status::ARCHIVED
                ],
                map_competition,
            )
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn create_competition_with_activation(
        &self,
        input: CreateCompetition,
        activate: bool,
    ) -> Result<Competition, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut created = self.create_competition(input)?;
        if activate {
            created = self.set_competition_status(&created.id, competition_status::ACTIVE)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(created)
    }

    pub fn create_competition(&self, input: CreateCompetition) -> Result<Competition, String> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err("Wettkampfname ist Pflicht".into());
        }
        let scoring = match input.scoring_mode.trim() {
            "teiler" => "teiler",
            _ => "ringe",
        };
        let max_shots = if input.max_shots <= 0 {
            10
        } else {
            input.max_shots
        };
        let nachkauf_enabled = input.nachkauf_enabled;
        let nachkauf_shots = if !nachkauf_enabled {
            0
        } else if input.nachkauf_shots < 0 {
            0
        } else {
            input.nachkauf_shots
        };
        let team_scoring_enabled = input.team_scoring_enabled;
        let team_count = if !team_scoring_enabled {
            3
        } else if input.team_count <= 0 {
            3
        } else {
            input.team_count
        };
        let kind = normalize_competition_kind(&input.kind);
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let date = if input.date.trim().is_empty() {
            Utc::now().date_naive().to_string()
        } else {
            input.date.trim().to_string()
        };
        let discipline = if input.discipline.trim().is_empty() {
            "Luftgewehr".to_string()
        } else {
            input.discipline.trim().to_string()
        };
        let status = competition_status::DRAFT;
        self.conn
            .execute(
                "INSERT INTO competitions
                 (id, name, date, discipline, max_shots, scoring_mode, status, created_at,
                  nachkauf_enabled, nachkauf_shots, team_scoring_enabled, team_count, kind,
                  tenths_enabled, probe_enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    id,
                    name,
                    date,
                    discipline,
                    max_shots,
                    scoring,
                    status,
                    created_at,
                    if nachkauf_enabled { 1 } else { 0 },
                    nachkauf_shots,
                    if team_scoring_enabled { 1 } else { 0 },
                    team_count,
                    kind,
                    if input.tenths_enabled { 1 } else { 0 },
                    if input.probe_enabled { 1 } else { 0 }
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(Competition {
            id,
            name: name.to_string(),
            date,
            discipline,
            max_shots,
            scoring_mode: scoring.to_string(),
            status: status.to_string(),
            created_at,
            kind: kind.to_string(),
            nachkauf_enabled,
            nachkauf_shots,
            team_scoring_enabled,
            team_count,
            tenths_enabled: input.tenths_enabled,
            probe_enabled: input.probe_enabled,
        })
    }

    pub fn set_competition_status(&self, id: &str, status: &str) -> Result<Competition, String> {
        let status = match status {
            competition_status::DRAFT
            | competition_status::ACTIVE
            | competition_status::CLOSED
            | competition_status::ARCHIVED
            | competition_status::TEMPLATE => status,
            _ => return Err("Ungültiger Wettkampfstatus".into()),
        };
        self.conn
            .execute(
                "UPDATE competitions SET status = ?1 WHERE id = ?2",
                params![status, id],
            )
            .map_err(|e| e.to_string())?;
        self.get_competition(id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".into())
    }

    /// Copy settings (and optionally start list + teams) from an existing competition.
    /// `as_template` → status `template`; otherwise `draft`. Empty name/date fall back to source / today.
    pub fn create_from_competition(
        &self,
        source_id: &str,
        name: Option<&str>,
        date: Option<&str>,
        as_template: bool,
        copy_entries: bool,
    ) -> Result<Competition, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let source = self
            .get_competition(source_id)?
            .ok_or_else(|| "Quell-Wettkampf nicht gefunden".to_string())?;
        let name = name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(source.name.as_str());
        let date = date
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if as_template {
                    source.date.clone()
                } else {
                    Utc::now().date_naive().to_string()
                }
            });
        let created = self.create_competition(CreateCompetition {
            name: name.to_string(),
            date,
            discipline: source.discipline.clone(),
            max_shots: source.max_shots,
            scoring_mode: source.scoring_mode.clone(),
            nachkauf_enabled: source.nachkauf_enabled,
            nachkauf_shots: source.nachkauf_shots,
            team_scoring_enabled: source.team_scoring_enabled,
            team_count: source.team_count,
            kind: source.kind.clone(),
            tenths_enabled: source.tenths_enabled,
            probe_enabled: source.probe_enabled,
        })?;
        let created = if as_template {
            self.set_competition_status(&created.id, competition_status::TEMPLATE)?
        } else {
            created
        };
        if copy_entries {
            let source_entries = self.list_entries(source_id)?;
            for entry in source_entries {
                self.add_entry(&created.id, &entry.person_id)?;
            }
            // Teams are global (person membership); no per-competition copy needed.
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(created)
    }

    pub fn set_competition_team_settings(
        &self,
        id: &str,
        team_scoring_enabled: bool,
        team_count: i64,
    ) -> Result<Competition, String> {
        let tx = rusqlite::Transaction::new_unchecked(&self.conn, rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let previous = self
            .get_competition(id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".to_string())?;
        let team_count = if !team_scoring_enabled {
            3
        } else if team_count <= 0 {
            3
        } else {
            team_count.min(20)
        };
        if previous.team_scoring_enabled != team_scoring_enabled || previous.team_count != team_count {
            self.assert_competition_rules_editable(id)?;
        }
        self.conn
            .execute(
                "UPDATE competitions
                 SET team_scoring_enabled = ?1, team_count = ?2
                 WHERE id = ?3",
                params![if team_scoring_enabled { 1 } else { 0 }, team_count, id],
            )
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        self.get_competition(id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".into())
    }

    pub fn update_competition(
        &self,
        id: &str,
        input: CreateCompetition,
    ) -> Result<Competition, String> {
        let tx = rusqlite::Transaction::new_unchecked(&self.conn, rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let previous = self
            .get_competition(id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".to_string())?;
        let name = input.name.trim();
        if name.is_empty() {
            return Err("Wettkampfname ist Pflicht".into());
        }
        let scoring = match input.scoring_mode.trim() {
            "teiler" => "teiler",
            _ => "ringe",
        };
        let max_shots = if input.max_shots <= 0 {
            10
        } else {
            input.max_shots
        };
        let nachkauf_enabled = input.nachkauf_enabled;
        let nachkauf_shots = 0i64;
        let team_scoring_enabled = input.team_scoring_enabled;
        let team_count = if !team_scoring_enabled {
            3
        } else if input.team_count <= 0 {
            3
        } else {
            input.team_count.min(20)
        };
        let kind = normalize_competition_kind(&input.kind);
        let date = if input.date.trim().is_empty() {
            Utc::now().date_naive().to_string()
        } else {
            input.date.trim().to_string()
        };
        let discipline = if input.discipline.trim().is_empty() {
            "Luftgewehr".to_string()
        } else {
            input.discipline.trim().to_string()
        };
        if previous.max_shots != max_shots || previous.scoring_mode != scoring
            || previous.nachkauf_enabled != nachkauf_enabled || previous.team_scoring_enabled != team_scoring_enabled
            || previous.team_count != team_count || previous.kind != kind || previous.discipline != discipline
            || previous.tenths_enabled != input.tenths_enabled || previous.probe_enabled != input.probe_enabled {
            self.assert_competition_rules_editable(id)?;
        }
        self.conn
            .execute(
                "UPDATE competitions SET
                   name = ?1,
                   date = ?2,
                   discipline = ?3,
                   max_shots = ?4,
                   scoring_mode = ?5,
                   nachkauf_enabled = ?6,
                   nachkauf_shots = ?7,
                   team_scoring_enabled = ?8,
                   team_count = ?9,
                   kind = ?10,
                   tenths_enabled = ?11,
                   probe_enabled = ?12
                 WHERE id = ?13",
                params![
                    name,
                    date,
                    discipline,
                    max_shots,
                    scoring,
                    if nachkauf_enabled { 1 } else { 0 },
                    nachkauf_shots,
                    if team_scoring_enabled { 1 } else { 0 },
                    team_count,
                    kind,
                    if input.tenths_enabled { 1 } else { 0 },
                    if input.probe_enabled { 1 } else { 0 },
                    id
                ],
            )
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        self.get_competition(id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".into())
    }

    fn assert_competition_rules_editable(&self, id: &str) -> Result<(), String> {
        let started: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE competition_id = ?1)", params![id], |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        if started { return Err("Wertungsregeln sind nach dem ersten Sessionstart unveränderlich — bitte neuen Wettkampf aus Vorlage erstellen".into()); }
        Ok(())
    }

    pub fn get_competition(&self, id: &str) -> Result<Option<Competition>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, date, discipline, max_shots, scoring_mode, status, created_at,
                        COALESCE(nachkauf_enabled, 0), COALESCE(nachkauf_shots, 0),
                        COALESCE(team_scoring_enabled, 0), COALESCE(team_count, 3),
                        COALESCE(kind, 'competition'), COALESCE(tenths_enabled, 1),
                        COALESCE(probe_enabled, 0)
                 FROM competitions WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], map_competition)
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    /// Effective shot limit for a competition session (= `max_shots` per series).
    pub fn effective_max_shots(
        &self,
        competition_id: &str,
        _entry_id: Option<&str>,
    ) -> Result<Option<i64>, String> {
        let Some(c) = self.get_competition(competition_id)? else {
            return Ok(None);
        };
        Ok(compute_effective_max_shots(c.max_shots))
    }

    /// Total scored shots across all sessions for a start-list entry.
    pub fn count_entry_shots(&self, entry_id: &str) -> Result<i64, String> {
        count_entry_shots_in_tx_conn(&self.conn, entry_id)
    }

    fn has_open_session_for_entry(&self, entry_id: &str) -> Result<bool, String> {
        let n: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sessions
                 WHERE entry_id = ?1 AND ended_at IS NULL",
                params![entry_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    fn count_ended_sessions_for_entry(&self, entry_id: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM sessions
                 WHERE entry_id = ?1 AND ended_at IS NOT NULL",
                params![entry_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    /// Start guard: open session blocks; `done` + Nachkauf allows a new full series;
    /// otherwise only the first series (no ended session yet).
    pub fn assert_entry_can_start(&self, entry_id: &str) -> Result<(), String> {
        let entry = self
            .get_entry(entry_id)?
            .ok_or_else(|| "Starter nicht gefunden".to_string())?;
        if self.has_open_session_for_entry(entry_id)? {
            return Err("Für diesen Starter läuft bereits eine offene Session".into());
        }
        let comp = self
            .get_competition(&entry.competition_id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".to_string())?;

        if entry.status == entry_status::DONE {
            if !comp.nachkauf_enabled {
                return Err("Schütze hat die Serie bereits beendet".into());
            }
            return Ok(());
        }

        let ended = self.count_ended_sessions_for_entry(entry_id)?;
        if ended > 0 {
            return Err(
                "Für diesen Starter existiert bereits eine Serie — erneutes Schießen nicht erlaubt"
                    .into(),
            );
        }
        Ok(())
    }

    pub fn list_entries(&self, competition_id: &str) -> Result<Vec<CompetitionEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT e.id, e.competition_id, e.person_id, e.start_order, e.status,
                        p.first_name, p.last_name, p.club,
                        COALESCE(e.nachkauf_purchased, 0)
                 FROM competition_entries e
                 JOIN people p ON p.id = e.person_id
                 WHERE e.competition_id = ?1
                 ORDER BY e.start_order ASC, p.last_name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![competition_id], map_entry)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn add_entry(
        &self,
        competition_id: &str,
        person_id: &str,
    ) -> Result<CompetitionEntry, String> {
        if self.get_competition(competition_id)?.is_none() {
            return Err("Wettkampf nicht gefunden".into());
        }
        if self.get_person(person_id)?.is_none() {
            return Err("Person nicht gefunden".into());
        }
        let next_order: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(start_order), 0) + 1 FROM competition_entries
                 WHERE competition_id = ?1",
                params![competition_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let id = Uuid::new_v4().to_string();
        let status = entry_status::WAITING;
        match self.conn.execute(
            "INSERT INTO competition_entries
             (id, competition_id, person_id, start_order, status, nachkauf_purchased)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![id, competition_id, person_id, next_order, status],
        ) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(info, _))
                if info.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err("Person ist bereits in der Startliste".into());
            }
            Err(e) => return Err(e.to_string()),
        }
        self.list_entries(competition_id)?
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| "Eintrag nicht lesbar".into())
    }

    /// Reorder start list: `entry_ids` is the full ordered list for the competition.
    pub fn reorder_entries(
        &self,
        competition_id: &str,
        entry_ids: &[String],
    ) -> Result<Vec<CompetitionEntry>, String> {
        let current = self.list_entries(competition_id)?;
        if current.len() != entry_ids.len() {
            return Err("Startliste unvollständig — bitte neu laden".into());
        }
        let valid: std::collections::HashSet<_> = current.iter().map(|e| e.id.as_str()).collect();
        let mut seen = std::collections::HashSet::new();
        for id in entry_ids {
            if !valid.contains(id.as_str()) {
                return Err("Ungültiger Starter in der Reihenfolge".into());
            }
            if !seen.insert(id.as_str()) {
                return Err("Doppelte Starter-ID".into());
            }
        }
        let tx = self.conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for (i, id) in entry_ids.iter().enumerate() {
            tx.execute(
                "UPDATE competition_entries SET start_order = ?1 WHERE id = ?2 AND competition_id = ?3",
                params![(i as i64) + 1, id, competition_id],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.list_entries(competition_id)
    }

    pub fn set_entry_status(
        &self,
        entry_id: &str,
        status: &str,
    ) -> Result<CompetitionEntry, String> {
        let status = match status {
            entry_status::WAITING
            | entry_status::PROBE
            | entry_status::ACTIVE
            | entry_status::DONE => status,
            _ => return Err("Ungültiger Starterstatus".into()),
        };
        if status == entry_status::ACTIVE {
            let entry = self.get_entry(entry_id)?.ok_or("Eintrag nicht gefunden")?;
            self.conn.execute(
                "UPDATE competition_entries
                 SET status = CASE WHEN id = ?1 THEN ?2 ELSE ?3 END
                 WHERE competition_id = ?4 AND (id = ?1 OR status = ?2)",
                params![entry_id, entry_status::ACTIVE, entry_status::WAITING, entry.competition_id],
            ).map_err(|e| e.to_string())?;
        } else {
            self.conn.execute(
                "UPDATE competition_entries SET status = ?1 WHERE id = ?2",
                params![status, entry_id],
            ).map_err(|e| e.to_string())?;
        }
        self.get_entry(entry_id)?
            .ok_or_else(|| "Eintrag nicht gefunden".into())
    }

    /// Mark previous active entries waiting/done transition: only one active per competition.
    /// On Nachkauf start (`done` → active), increments `nachkauf_purchased` (series counter).
    pub fn activate_entry(&self, entry_id: &str) -> Result<CompetitionEntry, String> {
        self.assert_entry_can_start(entry_id)?;
        let entry = self
            .get_entry(entry_id)?
            .ok_or_else(|| "Eintrag nicht gefunden".to_string())?;
        if entry.status == entry_status::DONE {
            self.conn
                .execute(
                    "UPDATE competition_entries
                     SET nachkauf_purchased = COALESCE(nachkauf_purchased, 0) + 1
                     WHERE id = ?1",
                    params![entry_id],
                )
                .map_err(|e| e.to_string())?;
        }
        self.mark_entry_active(entry_id)
    }

    /// Recovery resume: set entry active without start-guards (shots may already exist).
    pub fn reactivate_entry_for_resume(&self, entry_id: &str) -> Result<CompetitionEntry, String> {
        self.mark_entry_active(entry_id)
    }

    fn mark_entry_active(&self, entry_id: &str) -> Result<CompetitionEntry, String> {
        self.set_entry_status(entry_id, entry_status::ACTIVE)
    }

    pub fn get_entry(&self, entry_id: &str) -> Result<Option<CompetitionEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT e.id, e.competition_id, e.person_id, e.start_order, e.status,
                        p.first_name, p.last_name, p.club,
                        COALESCE(e.nachkauf_purchased, 0)
                 FROM competition_entries e
                 JOIN people p ON p.id = e.person_id
                 WHERE e.id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![entry_id], map_entry)
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn remove_entry(&self, entry_id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM competition_entries WHERE id = ?1",
                params![entry_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Clone start list person IDs from another competition (new waiting entries).
    pub fn clone_entries_from(
        &self,
        from_competition_id: &str,
        to_competition_id: &str,
    ) -> Result<Vec<CompetitionEntry>, String> {
        let source = self.list_entries(from_competition_id)?;
        for e in source {
            let _ = self.add_entry(to_competition_id, &e.person_id);
        }
        self.list_entries(to_competition_id)
    }
}

fn map_competition(row: &rusqlite::Row<'_>) -> rusqlite::Result<Competition> {
    let enabled: i64 = row.get(8)?;
    let team_enabled: i64 = row.get(10)?;
    let tenths: i64 = row.get(13)?;
    let probe_enabled: i64 = row.get(14)?;
    Ok(Competition {
        id: row.get(0)?,
        name: row.get(1)?,
        date: row.get(2)?,
        discipline: row.get(3)?,
        max_shots: row.get(4)?,
        scoring_mode: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        nachkauf_enabled: enabled != 0,
        nachkauf_shots: row.get(9)?,
        team_scoring_enabled: team_enabled != 0,
        team_count: row.get(11)?,
        kind: row.get(12)?,
        tenths_enabled: tenths != 0,
        probe_enabled: probe_enabled != 0,
    })
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<CompetitionEntry> {
    Ok(CompetitionEntry {
        id: row.get(0)?,
        competition_id: row.get(1)?,
        person_id: row.get(2)?,
        start_order: row.get(3)?,
        status: row.get(4)?,
        first_name: row.get(5)?,
        last_name: row.get(6)?,
        club: row.get(7)?,
        nachkauf_purchased: row.get(8)?,
    })
}

/// Per-series shot limit: `max_shots`, or `None` when unlimited (`max_shots <= 0`).
pub fn compute_effective_max_shots(max_shots: i64) -> Option<i64> {
    if max_shots <= 0 {
        None
    } else {
        Some(max_shots)
    }
}

/// Effective max shots inside an open transaction (Arena ingest).
pub fn effective_max_shots_in_tx(
    tx: &rusqlite::Transaction<'_>,
    competition_id: &str,
    _entry_id: Option<&str>,
) -> Result<Option<i64>, String> {
    let max_shots: i64 = tx
        .query_row(
            "SELECT max_shots FROM competitions WHERE id = ?1",
            params![competition_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("competition max_shots: {e}"))?;
    Ok(compute_effective_max_shots(max_shots))
}

/// Session → effective max (Arena ingest).
/// Prefers the persisted `sessions.max_shots` (set by the engine at session
/// start, incl. training series) so the limit holds inside the ingest TX.
/// `NULL` column: endless training, legacy pre-v13 sessions, or direct DB
/// test sessions → fall back to the competition limit (training: no limit).
pub fn session_effective_max_shots(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<Option<i64>, String> {
    let (session_max, competition_id): (Option<i64>, Option<String>) = tx
        .query_row(
            "SELECT max_shots, competition_id FROM sessions WHERE id = ?1",
            params![session_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("session for max_shots: {e}"))?;
    if let Some(max) = session_max {
        return Ok(compute_effective_max_shots(max));
    }
    let Some(cid) = competition_id else {
        return Ok(None);
    };
    effective_max_shots_in_tx(tx, &cid, None)
}

fn count_entry_shots_in_tx_conn(
    conn: &rusqlite::Connection,
    entry_id: &str,
) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*)
         FROM shots sh
         JOIN sessions s ON s.id = sh.session_id
         WHERE s.entry_id = ?1 AND sh.classification = 'scored'",
        params![entry_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

/// Shot count for the session limit — always this session only (each series has its own cap).
/// Probeschüsse (`classification = 'probe'`) never count toward the limit.
pub fn count_scored_shots_for_limit(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<i64, String> {
    tx.query_row(
        "SELECT COUNT(*) FROM shots WHERE session_id = ?1 AND classification = 'scored'",
        params![session_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

/// Whether the session scores in tenths. Training (no competition) → always tenths.
/// Missing / legacy competition column → tenths (preserve historical behaviour).
pub fn session_tenths_enabled(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
) -> Result<bool, String> {
    let tenths: i64 = tx
        .query_row(
            "SELECT COALESCE(json_extract(rules_json, '$.tenthsEnabled'), 1) FROM sessions WHERE id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("session for tenths: {e}"))?;
    Ok(tenths != 0)
}

#[cfg(test)]
mod rule_tests {
    use super::*;

    fn input(tenths: bool) -> CreateCompetition {
        serde_json::from_value(serde_json::json!({
            "name": "Fixture", "date": "2026-10-01", "discipline": "Luftgewehr",
            "maxShots": 10, "scoringMode": "ringe", "tenthsEnabled": tenths,
        })).unwrap()
    }

    #[test]
    fn rules_can_change_before_start_and_are_frozen_during_and_after_session() {
        let mut db = Database::open_in_memory().unwrap();
        let competition = db.create_competition(input(false)).unwrap();
        db.update_competition(&competition.id, input(true)).unwrap();
        let session = db.start_session("Fixture", Some(&competition.id), None, None).unwrap();
        for closed in [false, true] {
            if closed { db.end_session(&session.id).unwrap(); }
            assert!(db.update_competition(&competition.id, input(false)).unwrap_err().contains("unveränderlich"));
            assert!(db.set_competition_team_settings(&competition.id, true, 5).is_err());
            assert!(db.get_competition(&competition.id).unwrap().unwrap().tenths_enabled);
            let mut renamed = input(true);
            renamed.name = "Renamed".into();
            db.update_competition(&competition.id, renamed).unwrap();
        }
    }

    #[test]
    fn session_scoring_reads_snapshot_even_if_current_rules_change_outside_api() {
        let mut db = Database::open_in_memory().unwrap();
        let competition = db.create_competition(input(true)).unwrap();
        let session = db.start_session("Fixture", Some(&competition.id), None, None).unwrap();
        for current_tenths in [1, 0] {
            db.conn.execute("UPDATE competitions SET tenths_enabled=?1 WHERE id=?2", params![current_tenths, competition.id]).unwrap();
            let mut frame = crate::protocol::build_synthetic_shot_frame("09.5", "001.00", "00001", "00002").unwrap();
            crate::protocol::stamp_frame_nonce(&mut frame);
            let crate::arena::IngestOutcome::Accepted(accepted) = db.ingest_raw_frame(&session.id, &frame, "test", None).unwrap() else { panic!("shot rejected") };
            assert_eq!(accepted.score, 9.5);
        }
        let (json, origin): (String, String) = db.conn.query_row("SELECT rules_json, rules_origin FROM sessions WHERE id=?1", [&session.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(origin, "captured");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&json).unwrap()["tenthsEnabled"], 1);
    }

    #[test]
    fn legacy_migration_marks_current_baseline_without_recalculating_stored_shots() {
        let mut db = Database::open_in_memory().unwrap();
        let competition = db.create_competition(input(true)).unwrap();
        let session = db.start_session("Fixture", Some(&competition.id), None, None).unwrap();
        let frame = crate::protocol::build_synthetic_shot_frame("09.5", "001.00", "00001", "00002").unwrap();
        db.ingest_raw_frame(&session.id, &frame, "test", None).unwrap();
        db.conn.execute_batch("ALTER TABLE sessions DROP COLUMN rules_json;
            ALTER TABLE sessions DROP COLUMN rules_origin; DELETE FROM schema_migrations WHERE version=18;
            UPDATE competitions SET tenths_enabled=0;").unwrap();
        db.migrate().unwrap();
        let (origin, score): (String, f64) = db.conn.query_row(
            "SELECT rules_origin, (SELECT score FROM shots WHERE session_id=s.id LIMIT 1) FROM sessions s WHERE id=?1",
            [&session.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(origin, "legacy_current");
        assert_eq!(score, 9.5);
        assert!(db.update_competition(&competition.id, input(true)).is_err());
    }
}

#[cfg(test)]
mod atomic_entry_tests {
    use super::*;
    use crate::db::{CreatePerson, Database};

    #[test]
    fn reorder_rolls_back_on_second_update_and_active_switch_is_unique() {
        let db = Database::open_in_memory().unwrap();
        let comp = db.create_competition(serde_json::from_value(serde_json::json!({
            "name": "Atomic", "date": "2026-09-30", "discipline": "LG",
            "maxShots": 10, "scoringMode": "ringe"
        })).unwrap()).unwrap();
        let mut ids = Vec::new();
        for name in ["Erste", "Zweite"] {
            let person = db.create_person(CreatePerson {
                first_name: name.into(), last_name: "Test".into(), club: None,
            }).unwrap();
            ids.push(db.add_entry(&comp.id, &person.id).unwrap().id);
        }
        db.conn.execute_batch(&format!(
            "CREATE TRIGGER fail_reorder BEFORE UPDATE OF start_order ON competition_entries \
             WHEN NEW.id = '{}' BEGIN SELECT RAISE(ABORT, 'injected'); END;", ids[0]
        )).unwrap();
        assert!(db.reorder_entries(&comp.id, &[ids[1].clone(), ids[0].clone()]).is_err());
        let entries = db.list_entries(&comp.id).unwrap();
        assert_eq!(entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec![ids[0].as_str(), ids[1].as_str()]);
        db.conn.execute_batch("DROP TRIGGER fail_reorder;").unwrap();
        db.reorder_entries(&comp.id, &[ids[1].clone(), ids[0].clone()]).unwrap();
        db.reorder_entries(&comp.id, &[ids[1].clone(), ids[0].clone()]).unwrap();
        db.set_entry_status(&ids[0], entry_status::ACTIVE).unwrap();
        db.set_entry_status(&ids[1], entry_status::ACTIVE).unwrap();
        let entries = db.list_entries(&comp.id).unwrap();
        assert_eq!(entries.iter().filter(|e| e.status == entry_status::ACTIVE).count(), 1);
        assert_eq!(db.get_entry(&ids[1]).unwrap().unwrap().status, entry_status::ACTIVE);
    }
}
