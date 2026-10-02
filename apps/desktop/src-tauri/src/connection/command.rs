//! Commands into the connection owner thread.
//!
//! Product contract (Startup Nuclear):
//! - `Start`: Known BD_ADDR → one Startup Nuclear (PrimaryOnly).
//!   No Soft-Wake / silent Soft-RFCOMM in product.
//! - `NuclearLink`: Forget → Pair → RFCOMM (user Verbinden / Badge / Setup).
//! - Link-Lost → Idle (handled in owner keepalive; no Auto-Nuclear).

use super::connect_policy::ConnectOrigin;
use crate::transport::rfcomm::target::RfcommTarget;

#[derive(Debug, Clone)]
pub enum ConnectionCommand {
    Start,
    SelectTarget(RfcommTarget),
    /// User Verbinden / Setup: Forget → Pair → RFCOMM (blocking on owner).
    NuclearLink {
        bt_addr: u64,
        display_name: String,
        origin: ConnectOrigin,
    },
    ForgetTarget,
    RegisterSink {
        lease: u64,
        reply: std::sync::mpsc::Sender<Result<u64, String>>,
    },
    UnregisterSink {
        lease: u64,
        reply: std::sync::mpsc::Sender<bool>,
    },
    /// Session ACK / rare writes (manager owns ENQ).
    WriteBytes(Vec<u8>),
    /// Stop work (pairing / first-setup scan).
    PauseForSetup,
    /// Scan results may publish only while their acknowledged generation is current.
    SetupProgress {
        generation: u64,
        status: super::status::ConnectionStatus,
        reason: String,
    },
    /// Abort Nuclear (or other connect) in flight (bumps generation).
    CancelConnect,
    Shutdown,
}
