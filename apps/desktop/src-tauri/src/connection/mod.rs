//! Connection manager: owns RFCOMM socket (Startup Nuclear + Badge/Setup Nuclear).
//!
//! Commands never mention COM / PnP / radio.

#[cfg(test)]
mod backoff;
mod bridge;
mod command;
mod connect_policy;
pub use connect_policy::{BondLookup, ConnectOrigin, ConnectPhase};
pub(crate) mod diag;
mod event;
mod handle;
mod keepalive;
mod manager;
mod nuclear;
pub use nuclear::{forget_reddot_bonds, run_nuclear_link};
mod owner;
mod persist;
mod setup_flow;
mod shared;
mod sink;
pub(crate) mod shot_latency;
mod status;
mod timing;

#[cfg(test)]
mod product_owner_tests;
#[cfg(test)]
mod session_boundary_shot_gate_tests;

pub use crate::transport::rfcomm::target::{RfcommTarget, TargetSummary};
pub use command::ConnectionCommand;
pub use diag::DiagEventOwned;
pub use event::{ConnectionEvent, ConnectionEventKind};
pub use manager::{
    connect_known_nuclear, needs_setup, open_windows_bluetooth_settings, setup_connect, setup_scan,
    ConnectionHandle, ConnectionManager, RfcommBridgeTransport, SetupCandidate,
};
pub use persist::{
    clear_known_target, list_known_devices, load_known_target, remove_known_device,
    save_known_target, KnownDeviceSummary,
};
pub use status::ConnectionStatus;

/// Last N RFCOMM diag JSONL events (addresses anonymized).
pub fn diag_tail(data_dir: &std::path::Path, limit: usize) -> Vec<DiagEventOwned> {
    diag::tail(data_dir, limit)
}
