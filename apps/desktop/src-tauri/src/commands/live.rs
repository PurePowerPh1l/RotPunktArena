use crate::engine::{LiveState, StandEngine, StartSessionArgs};
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
pub fn get_live_state(engine: tauri::State<'_, Arc<StandEngine>>) -> LiveState {
    engine.snapshot()
}

#[tauri::command]
pub fn start_training(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    shooter_name: String,
    use_simulator: bool,
    person_id: Option<String>,
    endless: Option<bool>,
) -> Result<LiveState, String> {
    engine.start_session(
        app,
        StartSessionArgs {
            shooter_name,
            use_simulator,
            competition_id: None,
            entry_id: None,
            person_id,
            endless: endless.unwrap_or(false),
        },
    )
}

#[tauri::command]
pub fn start_entry_session(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    entry_id: String,
    use_simulator: bool,
) -> Result<LiveState, String> {
    let entry = engine
        .with_db(|db| {
            db.assert_entry_can_start(&entry_id)?;
            db.get_entry(&entry_id)
        })?
        .ok_or_else(|| "Starter nicht gefunden".to_string())?;
    let first = entry.first_name.unwrap_or_default();
    let last = entry.last_name.unwrap_or_default();
    let name = format!("{first} {last}").trim().to_string();
    engine.start_session(
        app,
        StartSessionArgs {
            shooter_name: name,
            use_simulator,
            competition_id: Some(entry.competition_id),
            entry_id: Some(entry.id),
            person_id: Some(entry.person_id),
            endless: false,
        },
    )
}

#[tauri::command]
pub fn end_training(engine: tauri::State<'_, Arc<StandEngine>>) -> Result<LiveState, String> {
    engine.end_session()
}

#[tauri::command]
/// „Wertung beginnen“ — end the probe phase and start the scored series.
pub fn finish_probe(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
) -> Result<LiveState, String> {
    engine.finish_probe(&app)
}

#[tauri::command]
pub fn queue_sim_shot(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    value_ascii: String,
    distance_ascii: String,
    x_ascii: String,
    y_ascii: String,
) -> Result<(), String> {
    engine
        .inject_synthetic_shot(&app, &value_ascii, &distance_ascii, &x_ascii, &y_ascii)
        .map(|_| ())
}

#[tauri::command]
pub fn fire_aim_shot(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<StandEngine>>,
    x: f64,
    y: f64,
) -> Result<LiveState, String> {
    engine.fire_aim_shot(&app, x, y)
}

#[tauri::command]
pub fn set_auto_fire(engine: tauri::State<'_, Arc<StandEngine>>, on: bool) {
    engine.set_auto_fire(on);
}

#[tauri::command]
/// Legacy COM list (feature `serial`); unused by RFCOMM UI.
pub fn list_serial_ports(engine: tauri::State<'_, Arc<StandEngine>>) -> Vec<String> {
    engine.list_ports()
}

#[tauri::command]
/// Legacy COM auto-detect (feature `serial`); unused by RFCOMM UI.
pub fn auto_detect_port(engine: tauri::State<'_, Arc<StandEngine>>) -> Option<String> {
    engine.auto_detect()
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RfcommStatusDto {
    pub status: String,
    pub reason: String,
    /// Paging phase: idle | paging | backoff | authStop.
    pub connect_phase: String,
    /// startupNuclear | badgeNuclear | setupNuclear | null
    pub connect_origin: Option<String>,
    pub generation: u64,
    pub target: Option<crate::connection::TargetSummary>,
    pub rfcomm_feature: bool,
    /// No authenticated RedDot bond — show first-setup sheet.
    pub needs_setup: bool,
}

#[tauri::command]
/// Snapshot of the app-lifetime RFCOMM owner.
///
/// # Preconditions
/// - `ConnectionHandle` registered at app start.
///
/// # Side effects
/// - None (read-only). Includes `needs_setup` for First-Setup sheet gating.
pub fn rfcomm_status(
    handle: tauri::State<'_, crate::connection::ConnectionHandle>,
) -> RfcommStatusDto {
    RfcommStatusDto {
        status: handle.status().as_str().to_string(),
        reason: handle.last_reason(),
        connect_phase: handle.connect_phase().as_str().to_string(),
        connect_origin: handle
            .connect_origin()
            .as_api_str()
            .map(|s| s.to_string()),
        generation: handle.generation(),
        target: handle.target().map(|t| t.summary()),
        rfcomm_feature: cfg!(feature = "rfcomm"),
        needs_setup: crate::connection::needs_setup(&handle),
    }
}

#[tauri::command]
/// First-Setup: discover a RedDot candidate (paired list, then inquiry).
///
/// # Preconditions
/// - Target should be powered and discoverable if not already bonded.
///
/// # Side effects
/// - Sends `PauseForSetup` (aborts in-flight connect). Inquiry (~seconds) runs
///   on a blocking worker so the command thread / UI stay responsive.
pub async fn rfcomm_setup_scan(
    handle: tauri::State<'_, crate::connection::ConnectionHandle>,
) -> Result<crate::connection::SetupCandidate, String> {
    let h = handle.inner().clone();
    tauri::async_runtime::spawn_blocking(move || crate::connection::setup_scan(&h))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
/// First-Setup: Nuclear link (Forget→Pair→RFCOMM), wait until `Linked`.
///
/// # Preconditions
/// - `bt_addr_hex` from a prior `rfcomm_setup_scan` (do not re-inquire here).
/// - Optional `display_name` avoids empty-name pairing UI.
///
/// # Side effects
/// - `PauseForSetup`; Owner `NuclearLink` (PIN 0000); persists known target.
///   Waits up to ~90s on a blocking worker (command thread stays responsive).
pub async fn rfcomm_setup_connect(
    handle: tauri::State<'_, crate::connection::ConnectionHandle>,
    bt_addr_hex: String,
    display_name: Option<String>,
) -> Result<crate::connection::TargetSummary, String> {
    let h = handle.inner().clone();
    let t = tauri::async_runtime::spawn_blocking(move || {
        crate::connection::setup_connect(&h, &bt_addr_hex, display_name.as_deref())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(t.summary())
}

#[tauri::command]
/// Drop persisted known target and abort the link.
///
/// # Preconditions
/// - None.
///
/// # Side effects
/// - `ForgetTarget`: clears `rfcomm_known_target.json`, bumps generation, → `needsTarget`.
/// - Primary recovery path out of `faulted` when persist/target is bad.
pub fn rfcomm_forget_target(
    handle: tauri::State<'_, crate::connection::ConnectionHandle>,
) -> Result<(), String> {
    handle.send(crate::connection::ConnectionCommand::ForgetTarget)
}

#[tauri::command]
/// Nuclear reconnect: Forget → Pair → RFCOMM (blocks until Linked or error).
pub fn rfcomm_reconnect(
    handle: tauri::State<'_, crate::connection::ConnectionHandle>,
) -> Result<crate::connection::TargetSummary, String> {
    let t = crate::connection::connect_known_nuclear(&handle)?;
    Ok(t.summary())
}

#[tauri::command]
/// Abort Nuclear (or other connect) in flight (bumps generation → cooperative cancel).
pub fn rfcomm_cancel_connect(
    handle: tauri::State<'_, crate::connection::ConnectionHandle>,
) -> Result<(), String> {
    handle.send(crate::connection::ConnectionCommand::CancelConnect)
}

#[tauri::command]
/// Last RFCOMM diag events for Support drawer (anonymized addresses).
pub fn rfcomm_diag_tail(
    app: tauri::AppHandle,
    limit: Option<u32>,
) -> Result<Vec<crate::connection::DiagEventOwned>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let n = limit.unwrap_or(12).clamp(1, 24) as usize;
    Ok(crate::connection::diag_tail(&data_dir, n))
}

#[tauri::command]
/// Historical: open Windows Bluetooth settings. Product pairing is in-app (PIN 0000).
///
/// # Preconditions
/// - None.
///
/// # Side effects
/// - Opens `ms-settings:bluetooth` via ShellExecute (fallback when in-app pair fails).
pub fn rfcomm_open_pairing_settings() -> Result<(), String> {
    crate::connection::open_windows_bluetooth_settings()
}
