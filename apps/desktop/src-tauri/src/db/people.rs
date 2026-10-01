use super::Database;
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub club: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePerson {
    pub first_name: String,
    pub last_name: String,
    pub club: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteTrainingShooterResult {
    pub person: Person,
    pub created: bool,
    pub linked_sessions: i64,
}

/// Split free-text shooter name into (first_name, last_name).
/// "Schütze 1" → ("Schütze", "1"); single token → (token, "—").
pub fn split_shooter_name(raw: &str) -> Result<(String, String), String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("Name ist Pflicht".into());
    }
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [] => Err("Name ist Pflicht".into()),
        [one] => Ok(((*one).to_string(), "—".to_string())),
        [first, rest @ ..] => Ok(((*first).to_string(), rest.join(" "))),
    }
}

impl Database {
    pub fn list_people(
        &self,
        query: Option<&str>,
        include_archived: bool,
    ) -> Result<Vec<Person>, String> {
        let q = query.map(str::trim).filter(|s| !s.is_empty());
        let mut out = Vec::new();
        if let Some(q) = q {
            let like = format!("%{q}%");
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, first_name, last_name, club, created_at,
                            COALESCE(archived, 0)
                     FROM people
                     WHERE (?1 = 1 OR COALESCE(archived, 0) = 0)
                       AND (first_name LIKE ?2 OR last_name LIKE ?2 OR IFNULL(club,'') LIKE ?2
                            OR (first_name || ' ' || last_name) LIKE ?2
                            OR (last_name || ', ' || first_name) LIKE ?2
                            OR (last_name || ' ' || first_name) LIKE ?2)
                     ORDER BY last_name COLLATE NOCASE, first_name COLLATE NOCASE",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(
                    params![if include_archived { 1i64 } else { 0i64 }, like],
                    map_person,
                )
                .map_err(|e| e.to_string())?;
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, first_name, last_name, club, created_at,
                            COALESCE(archived, 0)
                     FROM people
                     WHERE (?1 = 1 OR COALESCE(archived, 0) = 0)
                     ORDER BY last_name COLLATE NOCASE, first_name COLLATE NOCASE",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(
                    params![if include_archived { 1i64 } else { 0i64 }],
                    map_person,
                )
                .map_err(|e| e.to_string())?;
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
        }
        Ok(out)
    }

    pub fn create_person(&self, input: CreatePerson) -> Result<Person, String> {
        let first = input.first_name.trim();
        let last = input.last_name.trim();
        if first.is_empty() || last.is_empty() {
            return Err("Vor- und Nachname sind Pflicht".into());
        }
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let club = input
            .club
            .as_ref()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        self.conn
            .execute(
                "INSERT INTO people (id, first_name, last_name, club, created_at, archived)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                params![id, first, last, club, created_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(Person {
            id,
            first_name: first.to_string(),
            last_name: last.to_string(),
            club,
            created_at,
            archived: false,
        })
    }

    /// Find or create a person from free-text name and link matching training sessions.
    pub fn promote_training_shooter(
        &self,
        shooter_name: &str,
    ) -> Result<PromoteTrainingShooterResult, String> {
        let name = shooter_name.trim();
        if name.is_empty() {
            return Err("Name ist Pflicht".into());
        }

        let existing = self.find_person_by_display_name(name)?;
        let (person, created) = if let Some(p) = existing {
            (p, false)
        } else {
            let (first, last) = split_shooter_name(name)?;
            (
                self.create_person(CreatePerson {
                    first_name: first,
                    last_name: last,
                    club: None,
                })?,
                true,
            )
        };

        let linked = self.link_training_sessions_to_person(&person.id, name)?;
        Ok(PromoteTrainingShooterResult {
            person,
            created,
            linked_sessions: linked,
        })
    }

    fn find_person_by_display_name(&self, name: &str) -> Result<Option<Person>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, first_name, last_name, club, created_at, COALESCE(archived, 0)
                 FROM people
                 WHERE LOWER(TRIM(first_name || ' ' || last_name)) = LOWER(TRIM(?1))
                    OR (
                      last_name = '—'
                      AND LOWER(TRIM(first_name)) = LOWER(TRIM(?1))
                    )
                 ORDER BY COALESCE(archived, 0) ASC, created_at ASC
                 LIMIT 1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![name], map_person)
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    /// Attach orphan training sessions with the same free-text name to a person.
    pub fn link_training_sessions_to_person(
        &self,
        person_id: &str,
        shooter_name: &str,
    ) -> Result<i64, String> {
        let n = self
            .conn
            .execute(
                "UPDATE sessions
                 SET person_id = ?1,
                     shooter_name = (
                       SELECT TRIM(first_name || ' ' || CASE
                         WHEN last_name = '—' THEN ''
                         ELSE last_name
                       END)
                       FROM people WHERE id = ?1
                     )
                 WHERE person_id IS NULL
                   AND competition_id IS NULL
                   AND LOWER(TRIM(shooter_name)) = LOWER(TRIM(?2))",
                params![person_id, shooter_name.trim()],
            )
            .map_err(|e| e.to_string())?;
        Ok(n as i64)
    }

    pub fn set_person_archived(&self, id: &str, archived: bool) -> Result<Person, String> {
        let n = self
            .conn
            .execute(
                "UPDATE people SET archived = ?1 WHERE id = ?2",
                params![if archived { 1i64 } else { 0i64 }, id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Schütze nicht gefunden".into());
        }
        self.get_person(id)?
            .ok_or_else(|| "Schütze nicht gefunden".into())
    }

    pub fn update_person(&self, id: &str, input: CreatePerson) -> Result<Person, String> {
        let first = input.first_name.trim();
        let last = input.last_name.trim();
        if first.is_empty() || last.is_empty() {
            return Err("Vor- und Nachname sind Pflicht".into());
        }
        if self.get_person(id)?.is_none() {
            return Err("Schütze nicht gefunden".into());
        }
        let club = input
            .club
            .as_ref()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        self.conn
            .execute(
                "UPDATE people SET first_name = ?1, last_name = ?2, club = ?3 WHERE id = ?4",
                params![first, last, club, id],
            )
            .map_err(|e| e.to_string())?;

        let display = if last == "—" {
            first.to_string()
        } else {
            format!("{first} {last}")
        };
        self.conn
            .execute(
                "UPDATE sessions SET shooter_name = ?1 WHERE person_id = ?2",
                params![display, id],
            )
            .map_err(|e| e.to_string())?;

        self.get_person(id)?
            .ok_or_else(|| "Schütze nicht gefunden".into())
    }

    /// Permanently remove a shooter. Start-list entries and linked training
    /// sessions (incl. shots/frames/events) are removed.
    pub fn delete_person(&self, id: &str) -> Result<(), String> {
        // Reserve the writer before checking references: another connection
        // cannot start a linked session between the check and the deletion.
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )
        .map_err(|e| e.to_string())?;
        if self.get_person(id)?.is_none() {
            return Err("Schütze nicht gefunden".into());
        }
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions s
             WHERE s.ended_at IS NULL AND (s.person_id = ?1 OR s.entry_id IN
               (SELECT id FROM competition_entries WHERE person_id = ?1)))",
                params![id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if active {
            return Err("Schütze wird in einer offenen Session verwendet — zuerst beenden".into());
        }

        // Team memberships cascade via entry FK; clear entry refs on sessions first.
        let entry_ids: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM competition_entries WHERE person_id = ?1")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![id], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            let mut ids = Vec::new();
            for r in rows {
                ids.push(r.map_err(|e| e.to_string())?);
            }
            ids
        };

        for entry_id in &entry_ids {
            self.conn
                .execute(
                    "UPDATE sessions SET entry_id = NULL WHERE entry_id = ?1",
                    params![entry_id],
                )
                .map_err(|e| e.to_string())?;
        }

        self.conn
            .execute(
                "DELETE FROM competition_entries WHERE person_id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())?;

        self.delete_training_sessions_for_person(id)?;

        self.conn
            .execute(
                "UPDATE sessions SET person_id = NULL WHERE person_id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())?;

        let n = self
            .conn
            .execute("DELETE FROM people WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Schütze nicht gefunden".into());
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Hard-delete training sessions (and child rows) linked to this person.
    fn delete_training_sessions_for_person(&self, person_id: &str) -> Result<(), String> {
        let session_ids: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id FROM sessions
                     WHERE person_id = ?1 AND competition_id IS NULL",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![person_id], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            let mut ids = Vec::new();
            for r in rows {
                ids.push(r.map_err(|e| e.to_string())?);
            }
            ids
        };

        for sid in &session_ids {
            self.conn
                .execute("DELETE FROM shots WHERE session_id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
            self.conn
                .execute("DELETE FROM events WHERE session_id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
            self.conn
                .execute("DELETE FROM frames WHERE session_id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
            self.conn
                .execute("DELETE FROM sessions WHERE id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_person(&self, id: &str) -> Result<Option<Person>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, first_name, last_name, club, created_at, COALESCE(archived, 0)
                 FROM people WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], map_person)
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }
}

fn map_person(row: &rusqlite::Row<'_>) -> rusqlite::Result<Person> {
    let archived_i: i64 = row.get(5)?;
    Ok(Person {
        id: row.get(0)?,
        first_name: row.get(1)?,
        last_name: row.get(2)?,
        club: row.get(3)?,
        created_at: row.get(4)?,
        archived: archived_i != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deletion_fixture() -> (Database, String, String) {
        let mut db = Database::open_in_memory().unwrap();
        let person = db
            .create_person(CreatePerson {
                first_name: "Fixture".into(),
                last_name: "Deletion".into(),
                club: None,
            })
            .unwrap();
        let competition = db
            .create_competition(
                serde_json::from_value(serde_json::json!({
                    "name": "Fixture", "date": "2026-10-01", "discipline": "Luftgewehr",
                    "maxShots": 10, "scoringMode": "ringe"
                }))
                .unwrap(),
            )
            .unwrap();
        let entry = db.add_entry(&competition.id, &person.id).unwrap();
        let competition_session = db
            .start_session(
                "Fixture",
                Some(&competition.id),
                Some(&entry.id),
                Some(&person.id),
            )
            .unwrap();
        db.end_session(&competition_session.id).unwrap();
        let training = db
            .start_session("Fixture", None, None, Some(&person.id))
            .unwrap();
        let frame = crate::protocol::build_synthetic_shot_frame("10.0", "012.30", "00001", "00002")
            .unwrap();
        assert!(matches!(
            db.ingest_raw_frame(&training.id, &frame, "test", None)
                .unwrap(),
            crate::arena::IngestOutcome::Accepted(_)
        ));
        db.end_session(&training.id).unwrap();
        (db, person.id, competition_session.id)
    }

    fn deletion_counts(db: &Database) -> Vec<i64> {
        [
            "people",
            "competition_entries",
            "sessions",
            "shots",
            "events",
            "frames",
        ]
        .into_iter()
        .map(|table| {
            db.conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
    }

    #[test]
    fn person_deletion_rolls_back_every_step_and_retry_succeeds() {
        for operation in [
            "UPDATE OF entry_id ON sessions",
            "DELETE ON competition_entries",
            "DELETE ON shots",
            "DELETE ON events",
            "DELETE ON frames",
            "DELETE ON sessions",
            "UPDATE OF person_id ON sessions",
            "DELETE ON people",
        ] {
            let (db, person, competition_session) = deletion_fixture();
            let before = deletion_counts(&db);
            db.conn.execute_batch(&format!(
                "CREATE TRIGGER fail_delete BEFORE {operation} BEGIN SELECT RAISE(ABORT, 'injected'); END;"
            )).unwrap();
            assert!(db.delete_person(&person).is_err(), "{operation}");
            assert_eq!(deletion_counts(&db), before, "{operation}");
            assert!(db
                .get_session(&competition_session)
                .unwrap()
                .unwrap()
                .entry_id
                .is_some());
            db.conn.execute_batch("DROP TRIGGER fail_delete").unwrap();
            db.delete_person(&person).unwrap();
            assert!(db.get_person(&person).unwrap().is_none());
            assert!(db
                .get_session(&competition_session)
                .unwrap()
                .unwrap()
                .entry_id
                .is_none());
            assert_eq!(deletion_counts(&db), vec![0, 0, 1, 0, 2, 0]);
        }
    }

    #[test]
    fn person_deletion_rejects_open_direct_and_entry_references() {
        for entry_only in [false, true] {
            let (mut db, person, sid) = deletion_fixture();
            let previous = db.get_session(&sid).unwrap().unwrap();
            let session = db
                .start_session(
                    "Fixture",
                    previous.competition_id.as_deref(),
                    previous.entry_id.as_deref(),
                    if entry_only { None } else { Some(&person) },
                )
                .unwrap();
            let before = deletion_counts(&db);
            assert!(db
                .delete_person(&person)
                .unwrap_err()
                .contains("offenen Session"));
            assert_eq!(deletion_counts(&db), before);
            db.end_session(&session.id).unwrap();
            db.delete_person(&person).unwrap();
        }
    }

    #[test]
    fn split_two_tokens() {
        assert_eq!(
            split_shooter_name("Schütze 1").unwrap(),
            ("Schütze".into(), "1".into())
        );
    }

    #[test]
    fn split_single_token() {
        assert_eq!(
            split_shooter_name("Max").unwrap(),
            ("Max".into(), "—".into())
        );
    }

    #[test]
    fn split_rejects_empty() {
        assert!(split_shooter_name("   ").is_err());
    }

    #[test]
    fn promote_creates_person_and_links_sessions() {
        use crate::db::Database;

        let mut db = Database::open_in_memory().unwrap();
        let session = db
            .start_session("Schütze 1", None, None, None)
            .expect("session");

        let result = db.promote_training_shooter("Schütze 1").expect("promote");
        assert!(result.created);
        assert_eq!(result.person.first_name, "Schütze");
        assert_eq!(result.person.last_name, "1");
        assert_eq!(result.linked_sessions, 1);

        let again = db.promote_training_shooter("Schütze 1").expect("again");
        assert!(!again.created);
        assert_eq!(again.person.id, result.person.id);
        assert_eq!(again.linked_sessions, 0);

        let people = db.list_people(Some("Schütze"), false).unwrap();
        assert!(people.iter().any(|p| p.id == result.person.id));

        let linked: String = db
            .conn
            .query_row(
                "SELECT person_id FROM sessions WHERE id = ?1",
                rusqlite::params![session.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(linked, result.person.id);
    }

    #[test]
    fn update_and_delete_person() {
        use crate::db::Database;

        let mut db = Database::open_in_memory().unwrap();
        let person = db
            .create_person(CreatePerson {
                first_name: "Luke".into(),
                last_name: "Skywalker".into(),
                club: Some("Rebel".into()),
            })
            .unwrap();

        let updated = db
            .update_person(
                &person.id,
                CreatePerson {
                    first_name: "Anakin".into(),
                    last_name: "Skywalker".into(),
                    club: None,
                },
            )
            .unwrap();
        assert_eq!(updated.first_name, "Anakin");
        assert!(updated.club.is_none());

        let session = db
            .start_session("Anakin Skywalker", None, None, Some(&person.id))
            .unwrap();
        db.end_session(&session.id).unwrap();

        db.delete_person(&person.id).unwrap();
        assert!(db.get_person(&person.id).unwrap().is_none());

        let remaining: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE id = ?1 OR person_id = ?2",
                rusqlite::params![session.id, person.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);
    }
}
