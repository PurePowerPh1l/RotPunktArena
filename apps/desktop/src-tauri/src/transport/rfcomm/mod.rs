//! Native Winsock Bluetooth RFCOMM transport (AF_BTH).
//!
//! Replaces Virtual COM / BthModem. No serialport, PnP, or radio recovery.

pub mod auth_hook;
pub mod discovery;
pub mod error;
pub mod ffi;
pub mod runtime;
pub mod sdp;
pub mod socket;
pub mod spp_com;
pub mod target;

pub use error::TransportError;
pub use runtime::WinsockRuntime;
pub use socket::RfcommSocket;
pub use target::{RfcommTarget, SPP_SERVICE_UUID};

use std::time::Duration;

/// Byte-stream transport shared by RFCOMM socket and test doubles.
pub trait ByteTransport: Send {
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
    fn write_all(&mut self, data: &[u8], timeout: Duration) -> Result<(), TransportError>;
    #[allow(dead_code)]
    fn shutdown(&mut self) -> Result<(), TransportError>;
}
