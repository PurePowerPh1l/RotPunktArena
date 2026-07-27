//! Global teams (cross-competition) + team scoring.

use super::Database;
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionTeam {
    pub id: String,
    /// Kept for API compatibility; empty for global teams.
    #[serde(default)]
    pub competition_id: String,
    pub name: String,
    pub sort_order: i64,
    #[serde(default)]
    pub archived: bool,
    /// Global person membership.
    #[serde(default)]
    pub member_person_ids: Vec<String>,
    /// Entry IDs in a competition context (when listed with competition_id).
    #[serde(default)]
    pub member_entry_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberScore {
    pub entry_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub punkte_total: f64,
    pub teiler_avg: f64,
    pub shot_count: i64,
    pub counts: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamResultSummary {
    pub team_id: String,
    pub competition_id: String,
    pub name: String,
    pub sort_order: i64,
    pub member_count: i64,
    pub counting_members: i64,
    pub punkte_total: f64,
    pub teiler_sum: f64,
    pub teiler_avg: f64,
    pub members: Vec<TeamMemberScore>,
    pub rank_punkte: Option<i64>,
    pub rank_teiler: Option<i64>,
}

impl Database {
    /// List global teams. When `competition_id` is set, also fills `member_entry_ids`
    /// for people on the start list of that competition.
    pub fn list_teams(
        &self,
        competition_id: Option<&str>,
        include_archived: bool,
    ) -> Result<Vec<CompetitionTeam>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, sort_order, COALESCE(archived, 0)
                 FROM teams
                 WHERE (?1 = 1 OR archived = 0)
                 ORDER BY sort_order ASC, name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![if include_archived { 1i64 } else { 0i64 }], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (id, name, sort_order, archived) = row.map_err(|e| e.to_string())?;
            let member_person_ids = self.list_team_person_ids(&id)?;
            let member_entry_ids = if let Some(cid) = competition_id {
                self.list_team_entry_ids_for_competition(&id, cid)?
            } else {
                Vec::new()
            };
            out.push(CompetitionTeam {
                id,
                competition_id: competition_id.unwrap_or("").to_string(),
                name,
                sort_order,
                archived: archived != 0,
                member_person_ids,
                member_entry_ids,
            });
        }
        Ok(out)
    }

    /// Distinct team names (non-archived by default) — used as Arena suggestions.
    pub fn list_known_team_names(&self, include_archived: bool) -> Result<Vec<String>, String> {
        Ok(self
            .list_teams(None, include_archived)?
            .into_iter()
            .map(|t| t.name)
            .collect())
    }

    fn list_team_person_ids(&self, team_id: &str) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT tm.person_id
                 FROM team_members tm
                 JOIN people p ON p.id = tm.person_id
                 WHERE tm.team_id = ?1
                 ORDER BY p.last_name COLLATE NOCASE, p.first_name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![team_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    fn list_team_entry_ids_for_competition(
        &self,
        team_id: &str,
        competition_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT e.id
                 FROM competition_entries e
                 JOIN team_members tm ON tm.person_id = e.person_id
                 WHERE tm.team_id = ?1 AND e.competition_id = ?2
                 ORDER BY e.start_order ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![team_id, competition_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn create_team(&self, name: &str) -> Result<CompetitionTeam, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Teamname ist Pflicht".into());
        }
        if let Ok(existing) = self.conn.query_row(
            "SELECT id FROM teams WHERE name = ?1 COLLATE NOCASE AND archived = 0",
            params![name],
            |r| r.get::<_, String>(0),
        ) {
            return self
                .get_team(&existing, None)?
                .ok_or_else(|| "Team nicht lesbar".into());
        }
        let next_order: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM teams",
                [],
                |r| r.get(0),
            )
            .unwrap_or(1);
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO teams (id, name, archived, sort_order, created_at)
                 VALUES (?1, ?2, 0, ?3, ?4)",
                params![id, name, next_order, created_at],
            )
            .map_err(|e| {
                if matches!(
                    e,
                    rusqlite::Error::SqliteFailure(ref info, _)
                        if info.code == rusqlite::ErrorCode::ConstraintViolation
                ) {
                    "Teamname existiert bereits".into()
                } else {
                    e.to_string()
                }
            })?;
        self.get_team(&id, None)?
            .ok_or_else(|| "Team nicht lesbar".into())
    }

    /// Legacy signature used by clone — creates/returns global team by name (ignores competition_id).
    pub fn create_team_for_competition(
        &self,
        _competition_id: &str,
        name: &str,
    ) -> Result<CompetitionTeam, String> {
        self.create_team(name)
    }

    pub fn rename_team(&self, team_id: &str, name: &str) -> Result<CompetitionTeam, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Teamname ist Pflicht".into());
        }
        self.conn
            .execute(
                "UPDATE teams SET name = ?1 WHERE id = ?2",
                params![name, team_id],
            )
            .map_err(|e| e.to_string())?;
        self.get_team(team_id, None)?
            .ok_or_else(|| "Team nicht gefunden".into())
    }

    pub fn set_team_archived(&self, team_id: &str, archived: bool) -> Result<CompetitionTeam, String> {
        self.conn
            .execute(
                "UPDATE teams SET archived = ?1 WHERE id = ?2",
                params![if archived { 1 } else { 0 }, team_id],
            )
            .map_err(|e| e.to_string())?;
        self.get_team(team_id, None)?
            .ok_or_else(|| "Team nicht gefunden".into())
    }

    pub fn remove_team(&self, team_id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM team_members WHERE team_id = ?1", params![team_id])
            .map_err(|e| e.to_string())?;
        self.conn
            .execute("DELETE FROM teams WHERE id = ?1", params![team_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_team(
        &self,
        team_id: &str,
        competition_id: Option<&str>,
    ) -> Result<Option<CompetitionTeam>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, sort_order, COALESCE(archived, 0)
                 FROM teams WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![team_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next() {
            let (id, name, sort_order, archived) = row.map_err(|e| e.to_string())?;
            let member_person_ids = self.list_team_person_ids(&id)?;
            let member_entry_ids = if let Some(cid) = competition_id {
                self.list_team_entry_ids_for_competition(&id, cid)?
            } else {
                Vec::new()
            };
            Ok(Some(CompetitionTeam {
                id,
                competition_id: competition_id.unwrap_or("").to_string(),
                name,
                sort_order,
                archived: archived != 0,
                member_person_ids,
                member_entry_ids,
            }))
        } else {
            Ok(None)
        }
    }

    /// Assign person of the entry to the global team (moves if already on another team).
    pub fn add_team_member(
        &self,
        team_id: &str,
        entry_id: &str,
    ) -> Result<CompetitionTeam, String> {
        let entry = self
            .get_entry(entry_id)?
            .ok_or_else(|| "Starter nicht gefunden".to_string())?;
        self.add_team_person(team_id, &entry.person_id)?;
        self.get_team(team_id, Some(&entry.competition_id))?
            .ok_or_else(|| "Team nicht lesbar".into())
    }

    pub fn add_team_person(&self, team_id: &str, person_id: &str) -> Result<CompetitionTeam, String> {
        let _ = self
            .get_team(team_id, None)?
            .ok_or_else(|| "Team nicht gefunden".to_string())?;
        if self.get_person(person_id)?.is_none() {
            return Err("Person nicht gefunden".into());
        }
        // One team per person: clear previous membership.
        self.conn
            .execute(
                "DELETE FROM team_members WHERE person_id = ?1",
                params![person_id],
            )
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO team_members (team_id, person_id) VALUES (?1, ?2)",
                params![team_id, person_id],
            )
            .map_err(|e| e.to_string())?;
        self.get_team(team_id, None)?
            .ok_or_else(|| "Team nicht lesbar".into())
    }

    pub fn remove_team_member(
        &self,
        team_id: &str,
        entry_id: &str,
    ) -> Result<CompetitionTeam, String> {
        let entry = self
            .get_entry(entry_id)?
            .ok_or_else(|| "Starter nicht gefunden".to_string())?;
        self.remove_team_person(team_id, &entry.person_id)?;
        self.get_team(team_id, Some(&entry.competition_id))?
            .ok_or_else(|| "Team nicht gefunden".into())
    }

    pub fn remove_team_person(
        &self,
        team_id: &str,
        person_id: &str,
    ) -> Result<CompetitionTeam, String> {
        self.conn
            .execute(
                "DELETE FROM team_members WHERE team_id = ?1 AND person_id = ?2",
                params![team_id, person_id],
            )
            .map_err(|e| e.to_string())?;
        self.get_team(team_id, None)?
            .ok_or_else(|| "Team nicht gefunden".into())
    }

    /// Aggregate team scores for a competition from global membership ∩ start list.
    pub fn list_team_results(
        &self,
        competition_id: &str,
    ) -> Result<Vec<TeamResultSummary>, String> {
        let comp = self
            .get_competition(competition_id)?
            .ok_or_else(|| "Wettkampf nicht gefunden".to_string())?;
        if !comp.team_scoring_enabled {
            return Ok(Vec::new());
        }
        let team_count = comp.team_count.max(1) as usize;
        let teiler = comp.scoring_mode == "teiler";
        let individual = self.list_competition_results(competition_id)?;
        let by_entry: std::collections::HashMap<_, _> =
            individual.into_iter().map(|r| (r.entry_id.clone(), r)).collect();

        let teams = self.list_teams(Some(competition_id), false)?;
        let mut out = Vec::new();
        for team in teams {
            // Only include teams that have at least one starter in this competition,
            // or show all? Show teams with members present; also empty teams with 0.
            // Include all non-archived teams that have entry members OR skip empty for ranking.
            let mut members: Vec<TeamMemberScore> = team
                .member_entry_ids
                .iter()
                .filter_map(|eid| {
                    let r = by_entry.get(eid)?;
                    Some(TeamMemberScore {
                        entry_id: eid.clone(),
                        first_name: r.first_name.clone(),
                        last_name: r.last_name.clone(),
                        punkte_total: r.punkte_total,
                        teiler_avg: r.teiler_avg,
                        shot_count: r.shot_count,
                        counts: false,
                    })
                })
                .collect();

            if members.is_empty() && team.member_person_ids.is_empty() {
                continue;
            }
            // Skip teams with no one on this start list from podium noise? Keep if they have entries.
            if members.is_empty() {
                continue;
            }

            let mut ranked: Vec<usize> = members
                .iter()
                .enumerate()
                .filter(|(_, m)| m.shot_count > 0)
                .map(|(i, _)| i)
                .collect();
            ranked.sort_by(|&a, &b| {
                if teiler {
                    members[a]
                        .teiler_avg
                        .partial_cmp(&members[b].teiler_avg)
                        .unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    members[b]
                        .punkte_total
                        .partial_cmp(&members[a].punkte_total)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            });
            for &idx in ranked.iter().take(team_count) {
                members[idx].counts = true;
            }

            let counting: Vec<_> = members.iter().filter(|m| m.counts).collect();
            let punkte_total: f64 = counting.iter().map(|m| m.punkte_total).sum();
            let teiler_sum: f64 = counting.iter().map(|m| m.teiler_avg).sum();
            let counting_members = counting.len() as i64;
            let teiler_avg = if counting_members > 0 {
                teiler_sum / counting_members as f64
            } else {
                0.0
            };

            out.push(TeamResultSummary {
                team_id: team.id,
                competition_id: competition_id.to_string(),
                name: team.name,
                sort_order: team.sort_order,
                member_count: members.len() as i64,
                counting_members,
                punkte_total,
                teiler_sum,
                teiler_avg,
                members,
                rank_punkte: None,
                rank_teiler: None,
            });
        }

        assign_team_ranks(&mut out);
        out.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then_with(|| a.name.cmp(&b.name)));
        Ok(out)
    }
}

fn assign_team_ranks(rows: &mut [TeamResultSummary]) {
    let mut by_punkte: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, t)| t.counting_members > 0)
        .map(|(i, _)| i)
        .collect();
    by_punkte.sort_by(|&a, &b| {
        rows[b]
            .punkte_total
            .partial_cmp(&rows[a].punkte_total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| rows[a].name.cmp(&rows[b].name))
    });
    for (rank, &idx) in by_punkte.iter().enumerate() {
        rows[idx].rank_punkte = Some((rank + 1) as i64);
    }

    let mut by_teiler: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, t)| t.counting_members > 0)
        .map(|(i, _)| i)
        .collect();
    by_teiler.sort_by(|&a, &b| {
        rows[a]
            .teiler_sum
            .partial_cmp(&rows[b].teiler_sum)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| rows[a].name.cmp(&rows[b].name))
    });
    for (rank, &idx) in by_teiler.iter().enumerate() {
        rows[idx].rank_teiler = Some((rank + 1) as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(
        id: &str,
        name: &str,
        counting: i64,
        punkte: f64,
        teiler_sum: f64,
    ) -> TeamResultSummary {
        TeamResultSummary {
            team_id: id.into(),
            competition_id: "c1".into(),
            name: name.into(),
            sort_order: 0,
            member_count: counting,
            counting_members: counting,
            punkte_total: punkte,
            teiler_sum,
            teiler_avg: if counting > 0 {
                teiler_sum / counting as f64
            } else {
                0.0
            },
            members: vec![],
            rank_punkte: None,
            rank_teiler: None,
        }
    }

    #[test]
    fn team_ranks_punkte_and_teiler() {
        let mut rows = vec![
            team("t1", "Alpha", 3, 300.0, 90.0),
            team("t2", "Beta", 3, 330.0, 60.0),
            team("t3", "Gamma", 0, 0.0, 0.0),
        ];
        assign_team_ranks(&mut rows);
        let by_id = |id: &str| rows.iter().find(|t| t.team_id == id).unwrap();
        assert_eq!(by_id("t2").rank_punkte, Some(1));
        assert_eq!(by_id("t1").rank_punkte, Some(2));
        assert_eq!(by_id("t3").rank_punkte, None);
        assert_eq!(by_id("t2").rank_teiler, Some(1));
        assert_eq!(by_id("t1").rank_teiler, Some(2));
        assert_eq!(by_id("t3").rank_teiler, None);
    }
}
