//! Prepare a live session as one durable transition, before starting device I/O.
use super::{session_phase, Database, SessionInfo};

impl Database {
    pub fn start_live_session(
        &self,
        shooter_name: &str,
        competition_id: Option<&str>,
        entry_id: Option<&str>,
        person_id: Option<&str>,
        training_limit: Option<i64>,
    ) -> Result<(SessionInfo, Option<i64>, bool), String> {
        self.start_live_session_with_source(shooter_name, competition_id, entry_id, person_id, training_limit, false)
    }

    pub fn start_live_session_with_source(&self, shooter_name: &str, competition_id: Option<&str>,
        entry_id: Option<&str>, person_id: Option<&str>, training_limit: Option<i64>, simulated: bool,
    ) -> Result<(SessionInfo, Option<i64>, bool), String> {
        if simulated && competition_id.is_some() { return Err("Simulatortraining darf keinem Wettkampf zugeordnet werden".into()); }
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        if let Some(eid) = entry_id {
            let entry = self.get_entry(eid)?.ok_or("Starter nicht gefunden")?;
            if Some(entry.competition_id.as_str()) != competition_id
                || person_id.is_some_and(|id| id != entry.person_id)
            {
                return Err("Starter gehört nicht zu diesem Wettkampf/Schützen".into());
            }
            self.activate_entry(eid)?;
        }
        let (max_shots, probe) = match competition_id {
            Some(cid) => {
                let competition = self
                    .get_competition(cid)?
                    .ok_or("Wettkampf nicht gefunden")?;
                (
                    self.effective_max_shots(cid, entry_id)?,
                    competition.probe_enabled,
                )
            }
            None => (training_limit, false),
        };
        let phase = if probe {
            session_phase::PROBE
        } else {
            session_phase::MATCH
        };
        let mut session = Self::insert_session_in_tx(
            &tx,
            shooter_name,
            competition_id,
            entry_id,
            person_id,
            max_shots,
            phase,
        )?;
        tx.execute("UPDATE sessions SET simulated = ?1 WHERE id = ?2", rusqlite::params![simulated, session.id])
            .map_err(|e| e.to_string())?;
        session.simulated = simulated;
        tx.commit().map_err(|e| e.to_string())?;
        self.spawn_session_boundary_snapshot(&session.id);
        Ok((session, max_shots, probe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{entry_status, CreateCompetition, CreatePerson};

    #[test]
    fn simulator_source_is_persisted_and_competition_simulation_is_rejected() {
        let (db, cid, eid, pid) = fixture();
        assert!(db.start_live_session_with_source("Fixture", Some(&cid), Some(&eid), Some(&pid), None, true).is_err());
        let (session, _, _) = db.start_live_session_with_source("Fixture", None, None, None, Some(5), true).unwrap();
        assert!(session.simulated);
        assert!(db.get_session(&session.id).unwrap().unwrap().simulated);
        assert!(db.list_recovery_sessions().unwrap().iter().any(|s| s.id == session.id && s.simulated));
    }

    fn competition_input() -> CreateCompetition {
        serde_json::from_value(serde_json::json!({
            "name": "Test", "date": "2026-09-29", "discipline": "Luftgewehr",
            "maxShots": 10, "scoringMode": "ringe", "nachkaufEnabled": true,
            "probeEnabled": true
        }))
        .unwrap()
    }

    fn fixture() -> (Database, String, String, String) {
        let db = Database::open_in_memory().unwrap();
        let c = db.create_competition(competition_input()).unwrap();
        let p = db
            .create_person(CreatePerson {
                first_name: "Test".into(),
                last_name: "Starter".into(),
                club: None,
            })
            .unwrap();
        let e = db.add_entry(&c.id, &p.id).unwrap();
        db.set_entry_status(&e.id, entry_status::DONE).unwrap();
        (db, c.id, e.id, p.id)
    }

    #[test]
    fn failed_start_rolls_back_entry_counter_session_and_event_then_retry_succeeds() {
        let (db, cid, eid, pid) = fixture();
        db.conn.execute_batch("CREATE TRIGGER fail_start BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(db
            .start_live_session("Test", Some(&cid), Some(&eid), Some(&pid), None)
            .is_err());
        let entry = db.get_entry(&eid).unwrap().unwrap();
        assert_eq!(entry.status, entry_status::DONE);
        assert_eq!(entry.nachkauf_purchased, 0);
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        db.conn.execute_batch("DROP TRIGGER fail_start;").unwrap();
        let (session, max, probe) = db
            .start_live_session("Test", Some(&cid), Some(&eid), Some(&pid), None)
            .unwrap();
        assert_eq!(max, Some(10));
        assert!(probe);
        assert_eq!(db.get_session_max_shots(&session.id).unwrap(), Some(10));
        assert_eq!(
            db.get_session_phase(&session.id).unwrap(),
            session_phase::PROBE
        );
        assert_eq!(db.get_entry(&eid).unwrap().unwrap().nachkauf_purchased, 1);
        assert!(db
            .start_live_session("Test", Some(&cid), Some(&eid), Some(&pid), None)
            .is_err());
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(db.get_entry(&eid).unwrap().unwrap().nachkauf_purchased, 1);
    }

    #[test]
    fn training_limit_and_endless_are_persisted() {
        let db = Database::open_in_memory().unwrap();
        for limit in [Some(20), None] {
            let (s, max, probe) = db
                .start_live_session("Training", None, None, None, limit)
                .unwrap();
            assert_eq!(max, limit);
            assert!(!probe);
            assert_eq!(db.get_session_max_shots(&s.id).unwrap(), limit);
            assert_eq!(db.get_session_phase(&s.id).unwrap(), session_phase::MATCH);
        }
    }

    #[test]
    fn mismatched_entry_is_rejected_without_changes() {
        let (db, _, eid, pid) = fixture();
        assert!(db
            .start_live_session("Test", None, Some(&eid), Some(&pid), None)
            .is_err());
        assert_eq!(
            db.get_entry(&eid).unwrap().unwrap().status,
            entry_status::DONE
        );
    }

    #[test]
    fn competition_creation_and_activation_roll_back_together() {
        let db = Database::open_in_memory().unwrap();
        db.conn.execute_batch("CREATE TRIGGER fail_activate BEFORE UPDATE OF status ON competitions BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(db
            .create_competition_with_activation(competition_input(), true)
            .is_err());
        assert!(db.list_competitions(true).unwrap().is_empty());
        db.conn
            .execute_batch("DROP TRIGGER fail_activate;")
            .unwrap();
        let c = db
            .create_competition_with_activation(competition_input(), true)
            .unwrap();
        assert_eq!(c.status, "active");
        assert_eq!(db.list_competitions(true).unwrap().len(), 1);
    }

    #[test]
    fn template_copy_failure_leaves_no_partial_competition() {
        let (db, cid, _, _) = fixture();
        db.conn.execute_batch("CREATE TRIGGER fail_copy BEFORE INSERT ON competition_entries BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(db
            .create_from_competition(&cid, None, None, true, true)
            .is_err());
        assert_eq!(db.list_competitions(true).unwrap().len(), 1);
        db.conn.execute_batch("DROP TRIGGER fail_copy;").unwrap();
        let copied = db
            .create_from_competition(&cid, None, None, true, true)
            .unwrap();
        assert_eq!(copied.status, "template");
        assert_eq!(db.list_entries(&copied.id).unwrap().len(), 1);
    }
}
