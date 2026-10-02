//! ConnectionHandle — command sender + shared status snapshot.

use super::command::ConnectionCommand;
use super::connect_policy::{ConnectOrigin, ConnectPhase};
use super::shared::SharedState;
use super::sink::SinkChunk;
use super::status::ConnectionStatus;
use crate::transport::rfcomm::target::RfcommTarget;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

static NEXT_SINK_LEASE: AtomicU64 = AtomicU64::new(1);

/// Handle used by Tauri / session bridge.
#[derive(Clone)]
pub struct ConnectionHandle {
    pub(crate) cmd_tx: Sender<ConnectionCommand>,
    pub(crate) inner: Arc<Mutex<SharedState>>,
}

pub struct ConnectionSnapshot {
    pub status: ConnectionStatus,
    pub reason: String,
    pub generation: u64,
    pub target: Option<RfcommTarget>,
    pub phase: ConnectPhase,
    pub origin: ConnectOrigin,
}

impl ConnectionHandle {
    pub fn snapshot(&self) -> ConnectionSnapshot {
        let state = self.inner.lock().unwrap();
        ConnectionSnapshot {
            status: state.status,
            reason: state.last_reason.clone(),
            generation: state.generation,
            target: state.target.clone(),
            phase: state.connect_phase,
            origin: state.connect_origin,
        }
    }
    pub fn register_sink(&self) -> Result<(u64, u64), String> {
        let lease = NEXT_SINK_LEASE.fetch_add(1, Ordering::SeqCst);
        let (reply, receive) = std::sync::mpsc::channel();
        self.send(ConnectionCommand::RegisterSink { lease, reply })?;
        match receive.recv_timeout(Duration::from_millis(500)) {
            Ok(result) => result.map(|epoch| (lease, epoch)),
            Err(error) => {
                let (reply, _) = std::sync::mpsc::channel();
                if let Err(cleanup) = self.send(ConnectionCommand::UnregisterSink { lease, reply })
                {
                    eprintln!("Sink-Timeout-Bereinigung: {cleanup}");
                }
                Err(format!("Sink-Registrierung: {error}"))
            }
        }
    }

    pub fn unregister_sink(&self, lease: u64) -> Result<bool, String> {
        let (reply, receive) = std::sync::mpsc::channel();
        self.send(ConnectionCommand::UnregisterSink { lease, reply })?;
        receive
            .recv_timeout(Duration::from_millis(500))
            .map_err(|e| format!("Sink-Abmeldung: {e}"))
    }
    pub fn status(&self) -> ConnectionStatus {
        self.inner.lock().unwrap().status
    }

    pub fn generation(&self) -> u64 {
        self.inner.lock().unwrap().generation
    }

    pub fn target(&self) -> Option<RfcommTarget> {
        self.inner.lock().unwrap().target.clone()
    }

    pub fn last_reason(&self) -> String {
        self.inner.lock().unwrap().last_reason.clone()
    }

    /// Diag phase: `idle` / `paging` / `backoff` / `authStop`.
    pub fn connect_phase(&self) -> ConnectPhase {
        self.inner.lock().unwrap().connect_phase
    }

    pub fn connect_origin(&self) -> ConnectOrigin {
        self.inner.lock().unwrap().connect_origin
    }

    pub fn send(&self, cmd: ConnectionCommand) -> Result<(), String> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| "Connection manager gestoppt".to_string())
    }

    pub(crate) fn try_recv_sink_chunk(&self, epoch: u64) -> Option<SinkChunk> {
        let guard = self.inner.lock().unwrap();
        if !guard.sink_registered || guard.sink_epoch != epoch {
            return None;
        }
        let rx = guard.sink_rx.as_ref()?;
        rx.try_recv().ok()
    }
}
