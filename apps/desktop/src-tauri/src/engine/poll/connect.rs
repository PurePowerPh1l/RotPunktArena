//! Open DB + transport and emit initial Searching status.

use super::super::{emit_conn, ConnectionUpdate, StandEngine};
use crate::connection::{ConnectionHandle, RfcommBridgeTransport};
use crate::db::Database;
use crate::transport::simulator::{SimulatorControl, SimulatorTransport};
use crate::transport::{ConnectionStatus, Transport, TransportKind};
use tauri::{AppHandle, Manager};

pub(super) fn open_log(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    log_path: &std::path::Path,
    use_sim: bool,
) -> Option<Database> {
    match Database::open(log_path) {
        Ok(l) => Some(l),
        Err(e) => {
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Disconnected,
                    transport: if use_sim {
                        TransportKind::Simulator
                    } else {
                        TransportKind::Rfcomm
                    },
                    port: None,
                    detail: Some(e),
                },
            );
            None
        }
    }
}

pub(super) fn open_transport(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    log: &mut Database,
    use_sim: bool,
    last_port: Option<String>,
    sim_control: SimulatorControl,
) -> Option<Box<dyn Transport>> {
    if use_sim {
        let mut t = SimulatorTransport::new(sim_control);
        if let Err(e) = t.open() {
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Disconnected,
                    transport: TransportKind::Simulator,
                    port: None,
                    detail: Some(e.to_string()),
                },
            );
            return None;
        }
        return Some(Box::new(t));
    }

    let _ = last_port;
    let handle = match app.try_state::<ConnectionHandle>() {
        Some(h) => h.inner().clone(),
        None => {
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Disconnected,
                    transport: TransportKind::Rfcomm,
                    port: None,
                    detail: Some("RFCOMM ConnectionHandle fehlt".into()),
                },
            );
            return None;
        }
    };

    // Session uses the app-lifetime link only — no discover / force-connect here.
    let mut t = RfcommBridgeTransport::new(handle.clone());
    if let Err(e) = t.open() {
        emit_conn(
            app,
            engine,
            generation,
            ConnectionUpdate {
                status: ConnectionStatus::Disconnected,
                transport: TransportKind::Rfcomm,
                port: handle.target().map(|t| t.addr_hex()),
                detail: Some(e.to_string()),
            },
        );
        return None;
    }

    if let Some(target) = handle.target() {
        let _ = log.set_setting("last_rfcomm_addr", &target.addr_hex());
    }

    Some(Box::new(t))
}

pub(super) fn emit_searching(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    transport: &dyn Transport,
) {
    emit_conn(
        app,
        engine,
        generation,
        ConnectionUpdate {
            status: ConnectionStatus::Searching,
            transport: transport.kind(),
            port: Some(transport.name().to_string()),
            detail: Some("RFCOMM Link…".into()),
        },
    );
}
