//! ConnectionHandle — command sender + shared status snapshot.

use super::command::ConnectionCommand;
use super::connect_policy::{ConnectOrigin, ConnectPhase};
use super::shared::SharedState;
use super::sink::SinkChunk;
use super::status::ConnectionStatus;
use crate::transport::rfcomm::target::RfcommTarget;
use std::sync::mpsc::{Sender, TryRecvError};
use std::sync::{Arc, Mutex};

/// Handle used by Tauri / session bridge.
#[derive(Clone)]
pub struct ConnectionHandle {
    pub(crate) cmd_tx: Sender<ConnectionCommand>,
    pub(crate) inner: Arc<Mutex<SharedState>>,
}

impl ConnectionHandle {
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

    pub(crate) fn sink_registered(&self) -> bool {
        self.inner.lock().unwrap().sink_registered
    }

    pub(crate) fn sink_epoch(&self) -> u64 {
        self.inner.lock().unwrap().sink_epoch
    }

    pub fn send(&self, cmd: ConnectionCommand) -> Result<(), String> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| "Connection manager gestoppt".to_string())
    }

    pub(crate) fn try_recv_sink_chunk(&self) -> Option<SinkChunk> {
        let guard = self.inner.lock().unwrap();
        let rx = guard.sink_rx.as_ref()?;
        match rx.try_recv() {
            Ok(c) => Some(c),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}
