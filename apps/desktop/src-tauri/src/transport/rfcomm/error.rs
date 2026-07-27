//! RFCOMM / byte-transport errors (no COM terminology).

#![allow(dead_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("RFCOMM noch nicht implementiert: {0}")]
    NotImplemented(String),
    #[error("Timeout")]
    Timeout,
    #[error("Verbindung geschlossen")]
    Closed,
    #[error("Ziel nicht erreichbar")]
    TargetUnavailable,
    #[error("Remote getrennt")]
    RemoteDisconnected,
    #[error("Link reset")]
    LinkReset,
    #[error("Lokaler Shutdown")]
    LocalShutdown,
    #[error("Service abgelehnt")]
    ServiceRejected,
    #[error("Stack/Gerät geändert")]
    StackOrDeviceChanged,
    #[error("Nicht gepaart")]
    NotPaired,
    #[error("Service nicht gefunden")]
    ServiceNotFound,
    #[error("{0}")]
    Io(String),
    #[error("Winsock: {code} {message}")]
    Winsock { code: i32, message: String },
}

impl TransportError {
    pub fn winsock_code(&self) -> Option<i32> {
        match self {
            Self::Winsock { code, .. } => Some(*code),
            _ => None,
        }
    }
}

pub type RfcommError = TransportError;
