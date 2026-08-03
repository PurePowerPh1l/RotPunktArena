//! First-setup scan + nuclear connect (blocking helpers for Tauri commands).
//!
//! Start: Startup Nuclear once for Known BD_ADDR.
//! Sheet / Badge Verbinden → NuclearLink on owner (single-flight with Startup).

use super::command::ConnectionCommand;
use super::connect_policy::ConnectOrigin;
use super::handle::ConnectionHandle;
use super::status::ConnectionStatus;
use crate::transport::rfcomm::discovery::{bond_state, find_reddot_candidate, scan_all_reddots};
use crate::transport::rfcomm::target::RfcommTarget;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupCandidate {
    pub bt_addr_hex: String,
    pub display_name: String,
    pub already_paired: bool,
    /// True for the currently persisted (active) device.
    pub is_active: bool,
}

fn bond_authenticated(bt_addr: u64) -> bool {
    matches!(bond_state(bt_addr), Ok(Some(b)) if b.authenticated)
}

/// Sheet only when there is no known target (and not already Linked).
pub fn needs_setup(handle: &ConnectionHandle) -> bool {
    if handle.status() == ConnectionStatus::Linked {
        return false;
    }
    if handle.target().is_some() {
        return false;
    }
    true
}

fn set_shared_reason(handle: &ConnectionHandle, status: ConnectionStatus, reason: &str) {
    let mut g = handle.inner.lock().unwrap();
    g.status = status;
    g.last_reason = reason.to_string();
}

fn pause_owner_for_setup(handle: &ConnectionHandle) -> Result<(), String> {
    handle
        .send(ConnectionCommand::PauseForSetup)
        .map_err(|e| e)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if handle.status() == ConnectionStatus::Discovering {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(40));
    }
    Err("Owner PauseForSetup nicht bestätigt".into())
}

/// Pause connect + scan for **all** RedDots (paired list + nearby inquiry).
///
/// Always runs both sources so a new/second device shows up even while an
/// old one is still bonded. Active (persisted) device sorts first.
pub fn setup_scan(handle: &ConnectionHandle) -> Result<Vec<SetupCandidate>, String> {
    pause_owner_for_setup(handle)?;
    set_shared_reason(
        handle,
        ConnectionStatus::Discovering,
        "Suche RedDot in der Nähe…",
    );

    let active_addr = handle.target().map(|t| t.bt_addr & 0xFFFF_FFFF_FFFF);
    let devices = scan_all_reddots().map_err(|e| e.to_string())?;
    if devices.is_empty() {
        let msg = "Kein RedDot gefunden — Ziel einschalten, nah ans Gerät halten, erneut suchen"
            .to_string();
        set_shared_reason(handle, ConnectionStatus::NeedsTarget, &msg);
        return Err(msg);
    }

    let mut candidates: Vec<SetupCandidate> = devices
        .into_iter()
        .map(|d| {
            let addr = d.bt_addr & 0xFFFF_FFFF_FFFF;
            SetupCandidate {
                bt_addr_hex: format!("{addr:012X}"),
                display_name: d.display_name,
                already_paired: d.paired || bond_authenticated(addr),
                is_active: active_addr == Some(addr),
            }
        })
        .collect();
    // Active device first, discovery rank otherwise (stable sort).
    candidates.sort_by_key(|c| !c.is_active);

    let reason = if candidates.len() == 1 {
        format!("Gefunden: {}", candidates[0].display_name)
    } else {
        format!("{} RedDots gefunden", candidates.len())
    };
    set_shared_reason(handle, ConnectionStatus::Discovering, &reason);
    Ok(candidates)
}

fn parse_bt_addr_hex(hex: &str) -> Result<u64, String> {
    let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() != 12 {
        return Err(format!("Ungültige Bluetooth-Adresse: {hex}"));
    }
    u64::from_str_radix(&clean, 16).map_err(|e| e.to_string())
}

/// Wait for an in-flight or just-sent Nuclear to settle (single-flight attach).
fn wait_nuclear_outcome(
    handle: &ConnectionHandle,
    fallback: RfcommTarget,
    gen0: u64,
    attach: bool,
) -> Result<RfcommTarget, String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        let st = handle.status();
        let gen = handle.generation();
        if st == ConnectionStatus::Linked {
            // New Nuclear bumps gen; attach to Startup keeps the same gen on success.
            if attach || gen > gen0 {
                return Ok(handle.target().unwrap_or(fallback));
            }
        }
        if st == ConnectionStatus::Connecting {
            thread::sleep(Duration::from_millis(200));
            continue;
        }
        // Flight ended without link.
        if attach || gen > gen0 {
            if matches!(
                st,
                ConnectionStatus::Faulted
                    | ConnectionStatus::NeedsPairing
                    | ConnectionStatus::NeedsTarget
                    | ConnectionStatus::Idle
            ) {
                let reason = handle.last_reason();
                if st == ConnectionStatus::Idle
                    && !attach
                    && !reason.contains("Abgebrochen")
                    && !reason.contains("fehlgeschlagen")
                    && !reason.contains("Nicht verbunden")
                {
                    thread::sleep(Duration::from_millis(200));
                    continue;
                }
                return Err(if reason.is_empty() {
                    "Verbindung fehlgeschlagen".into()
                } else {
                    reason
                });
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    let msg = handle.last_reason();
    Err(if msg.is_empty() {
        "Zeitüberschreitung beim Verbinden".into()
    } else {
        msg
    })
}

/// Nuclear link: Forget → Pair → RFCOMM (same as Verbinden button).
pub fn setup_connect(
    handle: &ConnectionHandle,
    bt_addr_hex: &str,
    display_name_hint: Option<&str>,
) -> Result<RfcommTarget, String> {
    // If Startup Nuclear is already running, do not PauseForSetup (would cancel it).
    if handle.status() == ConnectionStatus::Connecting {
        let fallback = handle.target().unwrap_or(RfcommTarget {
            bt_addr: parse_bt_addr_hex(bt_addr_hex)?,
            display_name: display_name_hint.unwrap_or("RedDot").to_string(),
            service_uuid: crate::transport::rfcomm::SPP_SERVICE_UUID.to_string(),
            rfcomm_channel: Some(1),
            com_port: None,
        });
        return wait_nuclear_outcome(handle, fallback, handle.generation(), true);
    }

    pause_owner_for_setup(handle)?;

    let addr = parse_bt_addr_hex(bt_addr_hex)?;
    let mut display_name = display_name_hint
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("RedDot")
        .to_string();
    if display_name == "RedDot" {
        display_name = format!("RedDot {bt_addr_hex}");
    }
    if let Ok(Some(t)) = find_reddot_candidate() {
        if t.bt_addr == addr {
            display_name = t.display_name;
        }
    }

    set_shared_reason(
        handle,
        ConnectionStatus::Connecting,
        &format!("Verbinde mit {display_name}…"),
    );
    let gen0 = handle.generation();
    handle
        .send(ConnectionCommand::NuclearLink {
            bt_addr: addr,
            display_name: display_name.clone(),
            origin: ConnectOrigin::SetupNuclear,
        })
        .map_err(|e| e)?;

    wait_nuclear_outcome(
        handle,
        RfcommTarget {
            bt_addr: addr,
            display_name,
            service_uuid: crate::transport::rfcomm::SPP_SERVICE_UUID.to_string(),
            rfcomm_channel: Some(1),
            com_port: None,
        },
        gen0,
        false,
    )
}

/// Nuclear link for a known target (Verbinden button).
///
/// If Startup Nuclear is already `Connecting`, attaches to that flight — never a
/// second Forget/Pair sequence.
pub fn connect_known_nuclear(handle: &ConnectionHandle) -> Result<RfcommTarget, String> {
    let Some(t) = handle.target() else {
        return Err("Kein Ziel — „RedDot einrichten“".into());
    };

    if handle.status() == ConnectionStatus::Connecting {
        return wait_nuclear_outcome(handle, t, handle.generation(), true);
    }

    // Capture generation *before* send — Nuclear bumps generation on start.
    let gen0 = handle.generation();
    handle
        .send(ConnectionCommand::NuclearLink {
            bt_addr: t.bt_addr,
            display_name: t.display_name.clone(),
            origin: ConnectOrigin::BadgeNuclear,
        })
        .map_err(|e| e)?;

    wait_nuclear_outcome(handle, t, gen0, false)
}

pub fn open_windows_bluetooth_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        use ::windows::core::PCWSTR;
        use ::windows::Win32::UI::Shell::ShellExecuteW;
        use ::windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let url: Vec<u16> = "ms-settings:bluetooth\0".encode_utf16().collect();
        unsafe {
            let rc = ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if (rc.0 as isize) <= 32 {
                return Err(format!("ShellExecuteW failed ({})", rc.0 as isize));
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Bluetooth-Einstellungen nur unter Windows".into())
    }
}
