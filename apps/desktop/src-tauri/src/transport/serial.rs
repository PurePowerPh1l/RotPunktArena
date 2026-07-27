//! Real COM-port transport (feature = "serial").

use super::{probe_enq_nak, Transport, TransportKind};
use crate::protocol::BAUD_RATE;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io;
use std::time::Duration;

pub fn list_ports() -> Vec<String> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.port_name)
        .collect()
}

pub struct SerialTransport {
    port_name: String,
    port: Option<Box<dyn SerialPort>>,
}

impl SerialTransport {
    pub fn new(port_name: impl Into<String>) -> Self {
        Self {
            port_name: port_name.into(),
            port: None,
        }
    }
}

impl Transport for SerialTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Serial
    }

    fn name(&self) -> &str {
        &self.port_name
    }

    fn open(&mut self) -> io::Result<()> {
        let port = serialport::new(&self.port_name, BAUD_RATE)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        self.port = Some(port);
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.port = None;
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "port closed"))?;
        port.write_all(data)?;
        port.flush()?;
        Ok(())
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<usize> {
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "port closed"))?;
        port.set_timeout(timeout)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        match port.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e),
        }
    }
}

/// Try last successful port first, then others; ENQ → NAK/STX at 9600 8N1.
pub fn auto_detect(last_port: Option<&str>) -> Option<String> {
    let mut ports = list_ports();
    if ports.is_empty() {
        return None;
    }
    if let Some(last) = last_port {
        if let Some(idx) = ports.iter().position(|p| p == last) {
            let p = ports.remove(idx);
            ports.insert(0, p);
        }
    }

    for name in ports {
        let mut t = SerialTransport::new(&name);
        if t.open().is_err() {
            continue;
        }
        let ok = probe_enq_nak(&mut t, Duration::from_millis(500)).unwrap_or(false);
        let _ = t.close();
        if ok {
            return Some(name);
        }
    }
    None
}
