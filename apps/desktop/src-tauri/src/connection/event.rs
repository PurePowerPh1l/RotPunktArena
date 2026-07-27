//! Outbound connection events (status + reason).

use super::status::ConnectionStatus;
use crate::transport::rfcomm::target::TargetSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionEvent {
    pub kind: ConnectionEventKind,
    pub status: ConnectionStatus,
    pub reason: String,
    pub generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionEventKind {
    StatusChanged,
    Linked,
    LinkInterrupted,
    LinkResumed,
    NeedsPairing,
    Fault,
    TargetCleared,
    ShuttingDown,
}
