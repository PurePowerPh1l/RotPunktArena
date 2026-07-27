//! Shared status visible to Tauri / session bridge (behind ConnectionHandle).

use super::connect_policy::{ConnectOrigin, ConnectPhase};
use super::sink::SinkChunk;
use super::status::ConnectionStatus;
use crate::transport::rfcomm::target::RfcommTarget;
use std::sync::mpsc::Receiver;

pub(crate) struct SharedState {
    pub status: ConnectionStatus,
    pub generation: u64,
    pub target: Option<RfcommTarget>,
    pub last_reason: String,
    /// Paging / backoff / authStop for UI and diagnostics.
    pub connect_phase: ConnectPhase,
    /// Who started the current connect (startup auto vs nuclear).
    pub connect_origin: ConnectOrigin,
    pub sink_rx: Option<Receiver<SinkChunk>>,
    pub sink_registered: bool,
    /// Mirrors Owner `SinkFanout.epoch` after each Register/Unregister transition.
    pub sink_epoch: u64,
}
