//! Versioned, ordered, idempotent SQLite migrations.
//!
//! Intentional exception to the ~300-line guideline: this file is the single
//! migrations catalog (ordered list of schema versions). Splitting would hide
//! the sequence; length grows with each migration by design.

use rusqlite::Connection;

struct Migration {
    version: i64,
    name: &'static str,
    sql: Option<&'static str>,
    /// Custom migrator when SQL batch is insufficient (schema rewrites).
    custom: Option<fn(&Connection) -> Result<(), String>>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "baseline_sessions_events_settings",
        sql: Some(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
              id TEXT PRIMARY KEY,
              shooter_name TEXT NOT NULL,
              started_at TEXT NOT NULL,
              ended_at TEXT,
              competition_id TEXT,
              entry_id TEXT
            );
            CREATE TABLE IF NOT EXISTS events (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              created_at TEXT NOT NULL,
              payload TEXT NOT NULL,
              FOREIGN KEY(session_id) REFERENCES sessions(id)
            );
            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );
            "#,
        ),
        custom: None,
    },
    Migration {
        version: 2,
        name: "people_competitions",
        sql: Some(
            r#"
            CREATE TABLE IF NOT EXISTS people (
              id TEXT PRIMARY KEY,
              first_name TEXT NOT NULL,
              last_name TEXT NOT NULL,
              club TEXT,
              created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS competitions (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              date TEXT NOT NULL,
              discipline TEXT NOT NULL,
              max_shots INTEGER NOT NULL,
              scoring_mode TEXT NOT NULL,
              status TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS competition_entries (
              id TEXT PRIMARY KEY,
              competition_id TEXT NOT NULL,
              person_id TEXT NOT NULL,
              start_order INTEGER NOT NULL,
              status TEXT NOT NULL,
              FOREIGN KEY(competition_id) REFERENCES competitions(id),
              FOREIGN KEY(person_id) REFERENCES people(id),
              UNIQUE(competition_id, person_id)
            );
            "#,
        ),
        custom: None,
    },
    Migration {
        version: 3,
        name: "shot_integrity_frames_events_shots",
        sql: None,
        custom: Some(migrate_v3_integrity),
    },
    Migration {
        version: 4,
        name: "ensure_sessions_competition_columns",
        sql: None,
        custom: Some(migrate_v4_session_columns),
    },
    Migration {
        version: 5,
        name: "training_saved_flag",
        sql: None,
        custom: Some(migrate_v5_training_saved),
    },
    Migration {
        version: 6,
        name: "sessions_person_id",
        sql: None,
        custom: Some(migrate_v6_sessions_person),
    },
    Migration {
        version: 7,
        name: "competition_nachkauf",
        sql: None,
        custom: Some(migrate_v7_nachkauf),
    },
    Migration {
        version: 8,
        name: "competition_teams",
        sql: None,
        custom: Some(migrate_v8_teams),
    },
    Migration {
        version: 9,
        name: "session_autosave_marker",
        sql: None,
        custom: Some(migrate_v9_autosave),
    },
    Migration {
        version: 10,
        name: "people_archived",
        sql: None,
        custom: Some(migrate_v10_people_archived),
    },
    Migration {
        version: 11,
        name: "competition_kind",
        sql: None,
        custom: Some(migrate_v11_competition_kind),
    },
    Migration {
        version: 12,
        name: "global_teams",
        sql: None,
        custom: Some(migrate_v12_global_teams),
    },
    Migration {
        version: 13,
        name: "session_max_shots",
        sql: None,
        custom: Some(migrate_v13_session_max_shots),
    },
    Migration {
        version: 14,
        name: "hot_path_indices",
        sql: None,
        custom: Some(migrate_v14_hot_path_indices),
    },
    Migration {
        version: 15,
        name: "competition_tenths",
        sql: None,
        custom: Some(migrate_v15_competition_tenths),
    },
];

pub fn apply_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
          version INTEGER PRIMARY KEY,
          name TEXT NOT NULL,
          applied_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| e.to_string())?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for m in MIGRATIONS {
        if m.version <= current {
            continue;
        }
        conn.execute_batch("BEGIN IMMEDIATE;")
            .map_err(|e| e.to_string())?;
        let result = (|| {
            if let Some(sql) = m.sql {
                conn.execute_batch(sql)
                    .map_err(|e| format!("migration {} ({}): {e}", m.version, m.name))?;
            }
            if let Some(custom) = m.custom {
                custom(conn)?;
            }
            conn.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![m.version, m.name, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e);
            }
        }
    }
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({table})")) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let Ok(rows) = stmt.query_map([], |row| {
        let name: String = row.get(1)?;
        Ok(name)
    }) else {
        return false;
    };
    for r in rows.flatten() {
        if r == column {
            return true;
        }
    }
    false
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
        [table],
        |_| Ok(()),
    )
    .is_ok()
}

#[cfg(test)]
fn index_exists(conn: &Connection, index: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1",
        [index],
        |_| Ok(()),
    )
    .is_ok()
}

fn migrate_v3_integrity(conn: &Connection) -> Result<(), String> {
    if !table_has_column(conn, "sessions", "next_sequence") {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN next_sequence INTEGER NOT NULL DEFAULT 1",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    if !table_has_column(conn, "sessions", "recovery_state") {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN recovery_state TEXT NOT NULL DEFAULT 'clean'",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS frames (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          received_at TEXT NOT NULL,
          raw_frame_hex TEXT NOT NULL,
          frame_sha256 TEXT NOT NULL,
          parser_version TEXT NOT NULL,
          parse_status TEXT NOT NULL,
          device_sequence INTEGER,
          FOREIGN KEY(session_id) REFERENCES sessions(id),
          UNIQUE(session_id, frame_sha256)
        );
        CREATE INDEX IF NOT EXISTS idx_frames_session ON frames(session_id);
        "#,
    )
    .map_err(|e| e.to_string())?;

    // Rewrite events to sequenced schema if needed.
    if table_exists(conn, "events") && !table_has_column(conn, "events", "sequence") {
        if !table_exists(conn, "events_legacy_v2") {
            conn.execute("ALTER TABLE events RENAME TO events_legacy_v2", [])
                .map_err(|e| e.to_string())?;
        } else {
            conn.execute("DROP TABLE events", [])
                .map_err(|e| e.to_string())?;
        }
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS events (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          sequence INTEGER NOT NULL,
          kind TEXT NOT NULL,
          created_at TEXT NOT NULL,
          payload TEXT NOT NULL,
          actor_type TEXT NOT NULL,
          parser_version TEXT,
          FOREIGN KEY(session_id) REFERENCES sessions(id),
          UNIQUE(session_id, sequence)
        );
        CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);

        CREATE TABLE IF NOT EXISTS shots (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          frame_id TEXT NOT NULL UNIQUE,
          session_sequence INTEGER NOT NULL,
          shot_index INTEGER NOT NULL,
          score REAL NOT NULL,
          value_raw INTEGER NOT NULL,
          distance_raw INTEGER NOT NULL,
          x INTEGER NOT NULL,
          y INTEGER NOT NULL,
          classification TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY(session_id) REFERENCES sessions(id),
          FOREIGN KEY(frame_id) REFERENCES frames(id),
          UNIQUE(session_id, session_sequence)
        );
        CREATE INDEX IF NOT EXISTS idx_shots_session ON shots(session_id);
        "#,
    )
    .map_err(|e| e.to_string())?;

    if !table_has_column(conn, "events", "sequence") {
        return Err("events.sequence missing after v3 migration".into());
    }
    Ok(())
}

/// Legacy DBs created before competition_id existed: CREATE IF NOT EXISTS never added columns.
fn migrate_v4_session_columns(conn: &Connection) -> Result<(), String> {
    ensure_column(
        conn,
        "sessions",
        "competition_id",
        "ALTER TABLE sessions ADD COLUMN competition_id TEXT",
    )?;
    ensure_column(
        conn,
        "sessions",
        "entry_id",
        "ALTER TABLE sessions ADD COLUMN entry_id TEXT",
    )?;
    ensure_column(
        conn,
        "sessions",
        "next_sequence",
        "ALTER TABLE sessions ADD COLUMN next_sequence INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(
        conn,
        "sessions",
        "recovery_state",
        "ALTER TABLE sessions ADD COLUMN recovery_state TEXT NOT NULL DEFAULT 'clean'",
    )?;
    Ok(())
}

fn migrate_v5_training_saved(conn: &Connection) -> Result<(), String> {
    ensure_column(
        conn,
        "sessions",
        "training_saved",
        "ALTER TABLE sessions ADD COLUMN training_saved INTEGER NOT NULL DEFAULT 0",
    )?;
    Ok(())
}

fn migrate_v6_sessions_person(conn: &Connection) -> Result<(), String> {
    ensure_column(
        conn,
        "sessions",
        "person_id",
        "ALTER TABLE sessions ADD COLUMN person_id TEXT REFERENCES people(id)",
    )?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_sessions_person ON sessions(person_id);",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn migrate_v7_nachkauf(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "competitions") {
        return Ok(());
    }
    ensure_column(
        conn,
        "competitions",
        "nachkauf_enabled",
        "ALTER TABLE competitions ADD COLUMN nachkauf_enabled INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "competitions",
        "nachkauf_shots",
        "ALTER TABLE competitions ADD COLUMN nachkauf_shots INTEGER NOT NULL DEFAULT 0",
    )?;
    if table_exists(conn, "competition_entries") {
        ensure_column(
            conn,
            "competition_entries",
            "nachkauf_purchased",
            "ALTER TABLE competition_entries ADD COLUMN nachkauf_purchased INTEGER NOT NULL DEFAULT 0",
        )?;
    }
    Ok(())
}

fn migrate_v8_teams(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "competitions") || !table_exists(conn, "competition_entries") {
        return Ok(());
    }
    ensure_column(
        conn,
        "competitions",
        "team_scoring_enabled",
        "ALTER TABLE competitions ADD COLUMN team_scoring_enabled INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        conn,
        "competitions",
        "team_count",
        "ALTER TABLE competitions ADD COLUMN team_count INTEGER NOT NULL DEFAULT 3",
    )?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS competition_teams (
          id TEXT PRIMARY KEY,
          competition_id TEXT NOT NULL,
          name TEXT NOT NULL,
          sort_order INTEGER NOT NULL,
          FOREIGN KEY(competition_id) REFERENCES competitions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_competition_teams_comp
          ON competition_teams(competition_id);
        CREATE TABLE IF NOT EXISTS competition_team_members (
          team_id TEXT NOT NULL,
          entry_id TEXT NOT NULL,
          PRIMARY KEY (team_id, entry_id),
          UNIQUE(entry_id),
          FOREIGN KEY(team_id) REFERENCES competition_teams(id) ON DELETE CASCADE,
          FOREIGN KEY(entry_id) REFERENCES competition_entries(id) ON DELETE CASCADE
        );
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Granular autosave marker on sessions — Recovery Gate heartbeat without a second write path.
fn migrate_v9_autosave(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "sessions") {
        return Ok(());
    }
    ensure_column(
        conn,
        "sessions",
        "last_autosave_at",
        "ALTER TABLE sessions ADD COLUMN last_autosave_at TEXT",
    )?;
    ensure_column(
        conn,
        "sessions",
        "last_autosave_sequence",
        "ALTER TABLE sessions ADD COLUMN last_autosave_sequence INTEGER",
    )?;
    // Backfill open sessions from last event (or started_at) so Recovery Gate works after upgrade.
    if table_exists(conn, "events") {
        conn.execute_batch(
            r#"
            UPDATE sessions
            SET last_autosave_at = COALESCE(
                  (SELECT MAX(created_at) FROM events e WHERE e.session_id = sessions.id),
                  started_at
                ),
                last_autosave_sequence = COALESCE(
                  (SELECT MAX(sequence) FROM events e WHERE e.session_id = sessions.id),
                  0
                ),
                recovery_state = CASE
                  WHEN ended_at IS NULL AND recovery_state IN ('unclean', 'active') THEN 'active'
                  ELSE recovery_state
                END
            WHERE ended_at IS NULL
              AND (last_autosave_at IS NULL OR recovery_state = 'unclean');
            "#,
        )
        .map_err(|e| format!("v9 autosave backfill: {e}"))?;
    } else {
        conn.execute_batch(
            r#"
            UPDATE sessions
            SET last_autosave_at = COALESCE(last_autosave_at, started_at),
                last_autosave_sequence = COALESCE(last_autosave_sequence, 0)
            WHERE ended_at IS NULL AND last_autosave_at IS NULL;
            "#,
        )
        .map_err(|e| format!("v9 autosave backfill (no events): {e}"))?;
    }
    Ok(())
}

fn migrate_v10_people_archived(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "people") {
        return Ok(());
    }
    ensure_column(
        conn,
        "people",
        "archived",
        "ALTER TABLE people ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
    )?;
    Ok(())
}

fn migrate_v11_competition_kind(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "competitions") {
        return Ok(());
    }
    ensure_column(
        conn,
        "competitions",
        "kind",
        "ALTER TABLE competitions ADD COLUMN kind TEXT NOT NULL DEFAULT 'competition'",
    )?;
    Ok(())
}

/// Per-session shot limit so Arena ingest can enforce training series length
/// in the same TX (NULL = unlimited / endless / legacy pre-v13 session).
fn migrate_v13_session_max_shots(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "sessions") {
        return Ok(());
    }
    ensure_column(
        conn,
        "sessions",
        "max_shots",
        "ALTER TABLE sessions ADD COLUMN max_shots INTEGER",
    )?;
    Ok(())
}

/// Indices for hot-path filters that previously fell back to table scans:
/// results/series lookups by `entry_id`/`competition_id`, per-session shot
/// filtering by `classification`, entry lists per competition, and the ingest
/// dedupe probe on `(session_id, device_sequence)`. All `IF NOT EXISTS` and
/// guarded by table presence so this is safe on partial legacy schemas.
fn migrate_v14_hot_path_indices(conn: &Connection) -> Result<(), String> {
    if table_exists(conn, "sessions") {
        conn.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_entry ON sessions(entry_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_competition ON sessions(competition_id);
            "#,
        )
        .map_err(|e| format!("v14 sessions indices: {e}"))?;
    }
    if table_exists(conn, "shots") {
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_shots_session_classification
             ON shots(session_id, classification);",
        )
        .map_err(|e| format!("v14 shots index: {e}"))?;
    }
    if table_exists(conn, "frames") {
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_frames_session_devseq
             ON frames(session_id, device_sequence);",
        )
        .map_err(|e| format!("v14 frames index: {e}"))?;
    }
    if table_exists(conn, "competition_entries") {
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_competition_entries_competition
             ON competition_entries(competition_id);",
        )
        .map_err(|e| format!("v14 entries index: {e}"))?;
    }
    Ok(())
}

/// Zehntelwertung: new competitions default to whole rings (`0`); existing
/// rows are backfilled to `1` so historical results stay decimal.
fn migrate_v15_competition_tenths(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "competitions") {
        return Ok(());
    }
    ensure_column(
        conn,
        "competitions",
        "tenths_enabled",
        "ALTER TABLE competitions ADD COLUMN tenths_enabled INTEGER NOT NULL DEFAULT 0",
    )?;
    conn.execute(
        "UPDATE competitions SET tenths_enabled = 1",
        [],
    )
    .map_err(|e| format!("v15 tenths backfill: {e}"))?;
    Ok(())
}

/// Global teams (cross-competition) with person membership. Migrates legacy per-competition teams.
fn migrate_v12_global_teams(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "people") {
        return Ok(());
    }
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS teams (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          archived INTEGER NOT NULL DEFAULT 0,
          sort_order INTEGER NOT NULL DEFAULT 0,
          created_at TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_teams_name_nocase
          ON teams(name COLLATE NOCASE);
        CREATE TABLE IF NOT EXISTS team_members (
          team_id TEXT NOT NULL,
          person_id TEXT NOT NULL,
          PRIMARY KEY (team_id, person_id),
          UNIQUE(person_id),
          FOREIGN KEY(team_id) REFERENCES teams(id) ON DELETE CASCADE,
          FOREIGN KEY(person_id) REFERENCES people(id) ON DELETE CASCADE
        );
        "#,
    )
    .map_err(|e| e.to_string())?;

    // Migrate legacy competition_teams → global teams by distinct name.
    if table_exists(conn, "competition_teams") {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, sort_order FROM competition_teams
                 ORDER BY sort_order ASC, name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let legacy: Vec<(String, String, i64)> = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);

        use std::collections::HashMap;
        let mut name_to_id: HashMap<String, String> = HashMap::new();
        let now = chrono::Utc::now().to_rfc3339();
        for (legacy_id, name, sort_order) in &legacy {
            let key = name.trim().to_lowercase();
            if key.is_empty() {
                continue;
            }
            let global_id = if let Some(id) = name_to_id.get(&key) {
                id.clone()
            } else {
                let id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT OR IGNORE INTO teams (id, name, archived, sort_order, created_at)
                     VALUES (?1, ?2, 0, ?3, ?4)",
                    rusqlite::params![id, name.trim(), sort_order, now],
                )
                .map_err(|e| e.to_string())?;
                // If IGNORE fired due to race/unique, look up existing.
                let resolved: String = conn
                    .query_row(
                        "SELECT id FROM teams WHERE name = ?1 COLLATE NOCASE",
                        rusqlite::params![name.trim()],
                        |r| r.get(0),
                    )
                    .unwrap_or(id.clone());
                name_to_id.insert(key, resolved.clone());
                resolved
            };

            if table_exists(conn, "competition_team_members")
                && table_exists(conn, "competition_entries")
            {
                let mut mstmt = conn
                    .prepare(
                        "SELECT e.person_id
                         FROM competition_team_members m
                         JOIN competition_entries e ON e.id = m.entry_id
                         WHERE m.team_id = ?1",
                    )
                    .map_err(|e| e.to_string())?;
                let people: Vec<String> = mstmt
                    .query_map(rusqlite::params![legacy_id], |r| r.get::<_, String>(0))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(mstmt);
                for person_id in people {
                    // One team per person: skip if already assigned.
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO team_members (team_id, person_id) VALUES (?1, ?2)",
                        rusqlite::params![global_id, person_id],
                    );
                }
            }
        }
    }
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, sql: &str) -> Result<(), String> {
    if table_has_column(conn, table, column) {
        return Ok(());
    }
    conn.execute(sql, []).map_err(|e| format!("{table}.{column}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        apply_migrations(&conn).unwrap();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(v, 14);
        assert!(table_has_column(&conn, "events", "sequence"));
        assert!(table_has_column(&conn, "sessions", "next_sequence"));
        assert!(table_has_column(&conn, "sessions", "competition_id"));
        assert!(table_has_column(&conn, "sessions", "entry_id"));
        assert!(table_has_column(&conn, "sessions", "person_id"));
        assert!(table_has_column(&conn, "sessions", "training_saved"));
        assert!(table_has_column(&conn, "sessions", "last_autosave_at"));
        assert!(table_has_column(&conn, "sessions", "last_autosave_sequence"));
        assert!(table_has_column(&conn, "competitions", "nachkauf_enabled"));
        assert!(table_has_column(&conn, "competitions", "nachkauf_shots"));
        assert!(table_has_column(
            &conn,
            "competition_entries",
            "nachkauf_purchased"
        ));
        assert!(table_has_column(&conn, "competitions", "team_scoring_enabled"));
        assert!(table_has_column(&conn, "competitions", "team_count"));
        assert!(table_has_column(&conn, "competitions", "kind"));
        assert!(table_exists(&conn, "competition_teams"));
        assert!(table_exists(&conn, "competition_team_members"));
        assert!(table_exists(&conn, "teams"));
        assert!(table_exists(&conn, "team_members"));
        assert!(table_has_column(&conn, "people", "archived"));
        assert!(index_exists(&conn, "idx_sessions_entry"));
        assert!(index_exists(&conn, "idx_sessions_competition"));
        assert!(index_exists(&conn, "idx_shots_session_classification"));
        assert!(index_exists(&conn, "idx_frames_session_devseq"));
        assert!(index_exists(&conn, "idx_competition_entries_competition"));
        assert!(table_has_column(&conn, "competitions", "tenths_enabled"));
    }

    #[test]
    fn upgrades_legacy_sessions_without_competition_id() {
        let conn = Connection::open_in_memory().unwrap();
        // Simulate pre-bureau schema
        conn.execute_batch(
            r#"
            CREATE TABLE sessions (
              id TEXT PRIMARY KEY,
              shooter_name TEXT NOT NULL,
              started_at TEXT NOT NULL,
              ended_at TEXT
            );
            CREATE TABLE people (
              id TEXT PRIMARY KEY,
              first_name TEXT NOT NULL,
              last_name TEXT NOT NULL,
              club TEXT,
              created_at TEXT NOT NULL
            );
            CREATE TABLE schema_migrations (
              version INTEGER PRIMARY KEY,
              name TEXT NOT NULL,
              applied_at TEXT NOT NULL
            );
            INSERT INTO schema_migrations VALUES (1, 'legacy', '2020-01-01T00:00:00Z');
            INSERT INTO schema_migrations VALUES (2, 'legacy', '2020-01-01T00:00:00Z');
            INSERT INTO schema_migrations VALUES (3, 'legacy', '2020-01-01T00:00:00Z');
            "#,
        )
        .unwrap();
        assert!(!table_has_column(&conn, "sessions", "competition_id"));
        apply_migrations(&conn).unwrap();
        assert!(table_has_column(&conn, "sessions", "competition_id"));
        assert!(table_has_column(&conn, "sessions", "entry_id"));
        conn.execute(
            "INSERT INTO sessions (id, shooter_name, started_at, competition_id, entry_id)
             VALUES ('s1', 'T', '2020-01-01T00:00:00Z', NULL, NULL)",
            [],
        )
        .unwrap();
    }
}
