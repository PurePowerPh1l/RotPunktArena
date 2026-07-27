//! Append-only RFCOMM connection diagnostics (JSON lines).
//!
//! Log file: `<repo>/logs/rfcomm_connection.jsonl` (gitignored).
//! Falls back to `{data_dir}/rfcomm_connection.jsonl` if the repo path is unwritable.

use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub fn log_path(data_dir: &Path) -> PathBuf {
    repo_logs_dir()
        .map(|d| d.join("rfcomm_connection.jsonl"))
        .unwrap_or_else(|| data_dir.join("rfcomm_connection.jsonl"))
}

fn repo_logs_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../logs");
    create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Shared repo `logs/` directory for diagnose JSONL (RFCOMM + shot latency).
pub(crate) fn repo_logs_dir_for_diag() -> Option<PathBuf> {
    repo_logs_dir()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagEvent<'a> {
    pub ts: String,
    pub event: &'a str,
    pub status: &'a str,
    pub reason: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winsock: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winsock_name: Option<&'a str>,
    /// Startup Nuclear: always 1 when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_hook_installed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_attempted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<&'a str>,
    /// Known toast triggers (hook/release/B/pair/SDP) entered on Start?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toast_risk_path_entered: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagEventOwned {
    pub ts: String,
    pub event: String,
    pub status: String,
    pub reason: String,
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub addr: Option<String>,
    #[serde(default)]
    pub channel: Option<u32>,
    #[serde(default)]
    pub winsock: Option<i32>,
    #[serde(default)]
    pub winsock_name: Option<String>,
    #[serde(default)]
    pub attempt: Option<u32>,
    #[serde(default)]
    pub silent: Option<bool>,
    #[serde(default)]
    pub auth_hook_installed: Option<bool>,
    #[serde(default)]
    pub release_attempted: Option<bool>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub toast_risk_path_entered: Option<bool>,
}

pub fn winsock_name(code: i32) -> &'static str {
    match code {
        10013 => "WSAEACCES",
        10022 => "WSAEINVAL",
        10035 => "WSAEWOULDBLOCK",
        10048 => "WSAEADDRINUSE",
        10049 => "WSAEADDRNOTAVAIL",
        10050 => "WSAENETDOWN",
        10051 => "WSAENETUNREACH",
        10053 => "WSAECONNABORTED",
        10054 => "WSAECONNRESET",
        10057 => "WSAENOTCONN",
        10060 => "WSAETIMEDOUT",
        10061 => "WSAECONNREFUSED",
        10064 => "WSAEHOSTDOWN",
        10101 => "WSAEDISCON",
        _ => "WSA_OTHER",
    }
}

pub fn append(data_dir: &Path, ev: &DiagEvent<'_>) {
    let path = log_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = create_dir_all(parent);
    }
    let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    if let Ok(line) = serde_json::to_string(ev) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn append_repo(ev: &DiagEvent<'_>) {
    let Some(dir) = repo_logs_dir() else {
        return;
    };
    append(&dir, ev);
}

/// Full Startup-Nuclear soak line (observation fields; toast is manual).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupNuclearLog<'a> {
    pub ts: String,
    pub event: &'static str,
    /// Correlates JSONL row with handwritten matrix row (Toast notes).
    pub run_id: &'a str,
    pub origin: &'static str,
    pub generation: u64,
    pub target_bt_addr: &'a str,
    pub forget_scope: &'a str,
    pub forget_result: &'a str,
    pub forget_duration_ms: u64,
    pub pair_result: &'a str,
    pub pair_duration_ms: u64,
    pub auth_hook_installed: bool,
    pub auth_callback_count: u32,
    pub rfcomm_channel: Option<u32>,
    pub rfcomm_result: &'a str,
    pub linked: bool,
    pub duration_ms: u64,
    pub retry_scheduled: bool,
    /// Always null in product — operator records toast during soak.
    pub visible_toast_observed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_step: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winsock: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winsock_name: Option<&'a str>,
    pub cancelled: bool,
    pub next_state: &'a str,
    pub hook_deregistered: bool,
}

pub fn append_startup_nuclear(data_dir: &Path, ev: &StartupNuclearLog<'_>) {
    let path = log_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = create_dir_all(parent);
    }
    let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    if let Ok(line) = serde_json::to_string(ev) {
        let _ = writeln!(f, "{line}");
    }
}

pub fn now_ts() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Stable id for one Startup-Nuclear attempt (`r20260726T035412-g42`).
pub fn startup_run_id(generation: u64) -> String {
    format!(
        "r{}-g{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S"),
        generation
    )
}

fn anonymize_addr(addr: &str) -> String {
    let clean: String = addr.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() >= 8 {
        format!("{}…{}", &clean[..4], &clean[clean.len() - 4..])
    } else if clean.is_empty() {
        "—".into()
    } else {
        format!("{clean}…")
    }
}

/// Last `limit` JSONL events, newest last; addresses anonymized for Support UI.
pub fn tail(data_dir: &Path, limit: usize) -> Vec<DiagEventOwned> {
    let path = log_path(data_dir);
    let Ok(f) = OpenOptions::new().read(true).open(&path) else {
        return Vec::new();
    };
    let mut lines: Vec<String> = BufReader::new(f).lines().filter_map(|l| l.ok()).collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    lines
        .into_iter()
        .filter_map(|line| serde_json::from_str::<DiagEventOwned>(&line).ok())
        .map(|mut ev| {
            if let Some(a) = ev.addr.take() {
                ev.addr = Some(anonymize_addr(&a));
            }
            ev
        })
        .collect()
}
