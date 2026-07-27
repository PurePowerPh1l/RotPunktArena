//! Facade: public connection-manager API (split across sibling modules).
//!
//! See `docs/bluetooth-connection-stack.md`.

use super::command::ConnectionCommand;
use super::event::ConnectionEvent;
use super::owner::Owner;
use super::persist::load_known_target;
use super::shared::SharedState;
use super::status::ConnectionStatus;
use crate::transport::rfcomm::WinsockRuntime;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub use super::bridge::RfcommBridgeTransport;
pub use super::handle::ConnectionHandle;
pub use super::setup_flow::{
    connect_known_nuclear, needs_setup, open_windows_bluetooth_settings, setup_connect, setup_scan,
    SetupCandidate,
};

pub struct ConnectionManager {
    handle: ConnectionHandle,
    _join: JoinHandle<()>,
}

impl ConnectionManager {
    pub fn start(data_dir: PathBuf, event_tx: Option<Sender<ConnectionEvent>>) -> Self {
        let _ = WinsockRuntime::init();
        // PIN hook only around Nuclear (including Startup Nuclear).
        let known = load_known_target(&data_dir);
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (sink_tx, sink_rx) = mpsc::sync_channel::<super::sink::SinkChunk>(256);
        let inner = Arc::new(Mutex::new(SharedState {
            status: ConnectionStatus::Idle,
            generation: 0,
            target: known,
            last_reason: String::new(),
            connect_phase: super::connect_policy::ConnectPhase::Idle,
            connect_origin: super::connect_policy::ConnectOrigin::None,
            sink_rx: Some(sink_rx),
            sink_registered: false,
            sink_epoch: 0,
        }));
        let handle = ConnectionHandle {
            cmd_tx: cmd_tx.clone(),
            inner: Arc::clone(&inner),
        };
        let join = thread::Builder::new()
            .name("rfcomm-connection".into())
            .spawn(move || {
                Owner::new(data_dir, cmd_rx, sink_tx, inner, event_tx).run();
            })
            .expect("spawn rfcomm-connection");
        let _ = cmd_tx.send(ConnectionCommand::Start);
        Self {
            handle,
            _join: join,
        }
    }

    pub fn handle(&self) -> ConnectionHandle {
        self.handle.clone()
    }
}
