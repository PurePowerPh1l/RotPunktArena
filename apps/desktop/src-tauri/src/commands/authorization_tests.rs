//! Exercise the actual IPC dispatcher while backend admin state is locked.
use super::*;
use crate::{db::Database, engine::StandEngine};
use std::sync::Arc;
use tauri::Manager;

#[test]
fn locked_ipc_rejects_every_privileged_mutation_before_side_effects() {
    let engine = Arc::new(StandEngine::new(Database::open_in_memory().unwrap()));
    let app = tauri::test::mock_builder()
        .manage(engine.clone()).manage(AdminSession::default())
        .invoke_handler(tauri::generate_handler![
            create_person, update_person, delete_person, set_person_archived,
            create_competition, update_competition, set_competition_status,
            create_from_competition, set_competition_team_settings, add_entry,
            reorder_entries, set_entry_status, remove_entry, clone_entries,
            create_team, rename_team, set_team_archived, remove_team,
            add_team_member, remove_team_member, add_team_person, remove_team_person,
            clear_training_history, promote_training_shooter, restore_db_backup,
            reset_all_database, queue_sim_shot, fire_aim_shot, set_auto_fire,
            dev_inject_test_shot
        ]).build(tauri::test::mock_context(tauri::test::noop_assets())).unwrap();
    let view = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
    let body = serde_json::json!({
        "id": "fixture", "entryId": "fixture", "personId": "fixture", "teamId": "fixture",
        "competitionId": "fixture", "sourceId": "fixture", "fromCompetitionId": "fixture",
        "toCompetitionId": "fixture", "entryIds": [], "name": "fixture", "date": "2026-10-01",
        "archived": true, "status": "closed", "teamScoringEnabled": true, "teamCount": 3,
        "shooterName": "Fixture", "person": {"firstName": "Fixture", "lastName": "Test"},
        "competition": {"name": "Fixture", "date": "2026-10-01", "discipline": "Luftgewehr", "maxShots": 10, "scoringMode": "ringe"},
        "valueAscii": "10.0", "distanceAscii": "001.00", "xAscii": "00001", "yAscii": "00002",
        "x": 1, "y": 2, "on": true
    });
    for command in [
        "create_person", "update_person", "delete_person", "set_person_archived",
        "create_competition", "update_competition", "set_competition_status",
        "create_from_competition", "set_competition_team_settings", "add_entry",
        "reorder_entries", "set_entry_status", "remove_entry", "clone_entries",
        "create_team", "rename_team", "set_team_archived", "remove_team",
        "add_team_member", "remove_team_member", "add_team_person", "remove_team_person",
        "clear_training_history", "promote_training_shooter", "restore_db_backup",
        "reset_all_database", "queue_sim_shot", "fire_aim_shot", "set_auto_fire", "dev_inject_test_shot",
    ] {
        let response = tauri::test::get_ipc_response(&view, tauri::webview::InvokeRequest {
            cmd: command.into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(), body: tauri::ipc::InvokeBody::Json(body.clone()),
            headers: Default::default(), invoke_key: tauri::test::INVOKE_KEY.into(),
        });
        assert!(response.unwrap_err().as_str().unwrap().contains("Admin-Freigabe erforderlich"), "{command}");
    }
    assert!(engine.with_db(|db| db.list_people(None, true)).unwrap().is_empty());
    assert!(engine.with_db(|db| db.list_competitions(true)).unwrap().is_empty());
    assert_eq!(engine.with_db(|db| db.count_all_shots()).unwrap(), 0);
    // The same dispatcher permits the normal mutation once explicitly unlocked.
    app.state::<AdminSession>().unlock();
    let response = tauri::test::get_ipc_response(&view, tauri::webview::InvokeRequest {
        cmd: "create_person".into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(), body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(), invoke_key: tauri::test::INVOKE_KEY.into(),
    });
    assert!(response.is_ok());
    assert_eq!(engine.with_db(|db| db.list_people(None, true)).unwrap().len(), 1);
}
