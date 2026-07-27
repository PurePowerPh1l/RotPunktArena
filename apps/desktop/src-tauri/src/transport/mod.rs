//! Transport adapter layer — bytes in/out, independent of parser.

pub mod replay;
pub mod rfcomm;
#[cfg(feature = "serial")]
pub mod serial;
#[cfg(feature = "serial")]
pub mod serial_link;
pub mod simulator;

use std::io;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransportKind {
    Simulator,
    /// Legacy Virtual COM (feature `serial` only).
    Serial,
    /// Native Winsock Bluetooth RFCOMM.
    Rfcomm,
    /// Phase T — not implemented yet.
    Tcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Searching,
    Connected,
    Disconnected,
}

/// Byte-stream transport (RFCOMM / Simulator / legacy serial).
pub trait Transport: Send {
    fn kind(&self) -> TransportKind;
    fn name(&self) -> &str;
    fn open(&mut self) -> io::Result<()>;
    fn close(&mut self) -> io::Result<()>;
    fn write_all(&mut self, data: &[u8]) -> io::Result<()>;
    /// Non-blocking-ish read with timeout. Empty Ok means no data yet.
    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<usize>;
}

/// Probe a transport: send ENQ, expect NAK (or STX) within timeout.
#[allow(dead_code)]
pub fn probe_enq_nak(transport: &mut dyn Transport, timeout: Duration) -> io::Result<bool> {
    transport.write_all(&crate::protocol::encode_enq())?;
    let deadline = std::time::Instant::now() + timeout;
    let mut buf = [0u8; 64];
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match transport.read_timeout(&mut buf, remaining.min(Duration::from_millis(100))) {
            Ok(0) => continue,
            Ok(n) => {
                if buf[..n]
                    .iter()
                    .any(|&b| b == crate::protocol::NAK || b == crate::protocol::STX)
                {
                    return Ok(true);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(false)
}

/// List available serial port names (empty when `serial` feature is off).
pub fn list_serial_ports() -> Vec<String> {
    #[cfg(feature = "serial")]
    {
        serial::list_ports()
    }
    #[cfg(not(feature = "serial"))]
    {
        Vec::new()
    }
}

/// Auto-detect COM (legacy). Unused on RFCOMM path.
pub fn auto_detect(last_port: Option<&str>) -> Option<String> {
    #[cfg(feature = "serial")]
    {
        serial::auto_detect(last_port)
    }
    #[cfg(not(feature = "serial"))]
    {
        let _ = last_port;
        None
    }
}
