//! Connection / worker lifecycle emits for the poll loop.

use crate::db::Database;
use crate::transport::{ConnectionStatus, Transport};
use super::super::{emit_conn, ConnectionUpdate, StandEngine};
use tauri::AppHandle;

pub(super) fn on_nak(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    log: &mut Database,
    session_id: &str,
    transport: &dyn Transport,
    connected: &mut bool,
) {
    if !*connected {
        *connected = true;
        emit_conn(
            app,
            engine,
            generation,
            ConnectionUpdate {
                status: ConnectionStatus::Connected,
                transport: transport.kind(),
                port: Some(transport.name().to_string()),
                detail: None,
            },
        );
        let _ = log.append_event(
            session_id,
            "connection",
            "device",
            serde_json::json!({
                "status": "connected",
                "transport": format!("{:?}", transport.kind()).to_lowercase(),
                "port": transport.name(),
            }),
        );
    }
}

pub(super) fn ensure_connected(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    transport: &dyn Transport,
    connected: &mut bool,
) {
    if !*connected {
        *connected = true;
        emit_conn(
            app,
            engine,
            generation,
            ConnectionUpdate {
                status: ConnectionStatus::Connected,
                transport: transport.kind(),
                port: Some(transport.name().to_string()),
                detail: None,
            },
        );
    }
}

pub(super) fn worker_ended(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    transport: &dyn Transport,
) {
    if engine.generation.load(std::sync::atomic::Ordering::SeqCst) == generation {
        emit_conn(
            app,
            engine,
            generation,
            ConnectionUpdate {
                status: ConnectionStatus::Disconnected,
                transport: transport.kind(),
                port: Some(transport.name().to_string()),
                detail: Some("Worker beendet".into()),
            },
        );
    }
}
