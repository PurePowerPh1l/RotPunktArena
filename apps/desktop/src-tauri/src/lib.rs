mod arena;
mod commands;
pub mod connection;
mod db;
mod engine;
mod protocol;
mod transport;

use commands::AdminSession;
use connection::{ConnectionCommand, ConnectionHandle, ConnectionManager};
use db::Database;
use engine::StandEngine;
use std::sync::Arc;
use tauri::{Manager, RunEvent, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let db_path = data_dir.join("reddot.sqlite");
            let log = Database::open(&db_path).map_err(|e| e.to_string())?;
            let engine = Arc::new(StandEngine::new(log));
            // RFCOMM owner thread — autoconnect known/paired BD_ADDR; no COM/PnP/radio.
            let mgr = ConnectionManager::start(data_dir.clone(), None);
            let handle = mgr.handle();
            app.manage(engine);
            app.manage(handle);
            // Keep manager alive for process lifetime.
            app.manage(mgr);
            // Server-side admin unlock flag (starts locked; UI unlock sets it).
            app.manage(AdminSession::default());
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(
                event,
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
            ) {
                request_rfcomm_shutdown(&window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_live_state,
            commands::start_training,
            commands::start_entry_session,
            commands::end_training,
            commands::finish_probe,
            commands::queue_sim_shot,
            commands::fire_aim_shot,
            commands::set_auto_fire,
            commands::list_serial_ports,
            commands::auto_detect_port,
            commands::rfcomm_status,
            commands::rfcomm_setup_scan,
            commands::rfcomm_setup_connect,
            commands::rfcomm_forget_target,
            commands::rfcomm_list_devices,
            commands::rfcomm_forget_device,
            commands::rfcomm_reconnect,
            commands::rfcomm_cancel_connect,
            commands::rfcomm_diag_tail,
            commands::rfcomm_open_pairing_settings,
            commands::list_people,
            commands::create_person,
            commands::update_person,
            commands::delete_person,
            commands::set_person_archived,
            commands::list_competitions,
            commands::create_competition,
            commands::update_competition,
            commands::create_from_competition,
            commands::set_competition_status,
            commands::set_competition_team_settings,
            commands::list_entries,
            commands::add_entry,
            commands::reorder_entries,
            commands::set_entry_status,
            commands::set_entry_nachkauf,
            commands::remove_entry,
            commands::clone_entries,
            commands::list_competition_results,
            commands::get_entry_result,
            commands::list_entry_series,
            commands::list_teams,
            commands::list_known_team_names,
            commands::create_team,
            commands::rename_team,
            commands::set_team_archived,
            commands::remove_team,
            commands::add_team_member,
            commands::remove_team_member,
            commands::add_team_person,
            commands::remove_team_person,
            commands::list_team_results,
            commands::dev_diagnostics,
            commands::dev_inject_test_shot,
            commands::create_db_backup,
            commands::list_db_backups,
            commands::restore_db_backup,
            commands::reset_all_database,
            commands::get_ui_prefs,
            commands::set_ui_prefs,
            commands::get_admin_auth_status,
            commands::setup_admin_password,
            commands::verify_admin_password,
            commands::lock_admin_session,
            commands::dev_unlock_admin_session,
            commands::reset_training_series,
            commands::set_training_endless,
            commands::set_training_series_shots,
            commands::save_training_session,
            commands::list_training_history,
            commands::get_training_session_detail,
            commands::list_training_shooters,
            commands::clear_training_history,
            commands::promote_training_shooter,
            commands::list_recovery_sessions,
            commands::close_interrupted_session,
            commands::abandon_session,
            commands::resume_session,
            commands::export_diagnostics,
            commands::export_emergency_bundle,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
                request_rfcomm_shutdown(app);
            }
        });
}

fn request_rfcomm_shutdown(app: &tauri::AppHandle) {
    if let Some(handle) = app.try_state::<ConnectionHandle>() {
        let _ = handle.send(ConnectionCommand::Shutdown);
    }
}

/// Library surface for integration tests (Arena Core).
pub use arena::{AcceptedShot, IngestOutcome, PARSER_VERSION};
pub use db::{
    CreateCompetition, CreatePerson, Database as ArenaDb, RecoverySessionInfo, StoredUiShot,
    SNAPSHOT_EVERY_N_SHOTS, SNAPSHOT_SUBDIR,
};
pub use protocol::{
    build_synthetic_shot_frame, encode_ack, encode_enq, stamp_frame_nonce, Incoming,
    RedDotStreamParser,
};
pub use transport::replay::{parse_hex_capture, ReplayTransport};
pub use transport::rfcomm;
pub use transport::Transport;
