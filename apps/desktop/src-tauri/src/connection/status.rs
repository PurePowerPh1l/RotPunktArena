//! Connection status for UI / diagnostics.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    #[default]
    Idle,
    NeedsTarget,
    Discovering,
    Connecting,
    Linked,
    NeedsPairing,
    Faulted,
    ShuttingDown,
}

impl ConnectionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::NeedsTarget => "needsTarget",
            Self::Discovering => "discovering",
            Self::Connecting => "connecting",
            Self::Linked => "linked",
            Self::NeedsPairing => "needsPairing",
            Self::Faulted => "faulted",
            Self::ShuttingDown => "shuttingDown",
        }
    }
}
