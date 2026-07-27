//! Replay transport — feeds golden `.hex` captures through the same ENQ/read path as live.

use super::{Transport, TransportKind};
use crate::protocol::{ACK, ENQ, NAK};
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::time::Duration;

/// Parses docs/captures `.hex` format into RX byte chunks (one per RX line / contiguous RX).
pub fn parse_hex_capture(text: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut chunks = Vec::new();
    for (lineno, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (dir, hex_part) = if let Some(rest) = line.strip_prefix("RX:") {
            ("RX", rest)
        } else if let Some(rest) = line.strip_prefix('<') {
            ("RX", rest)
        } else if let Some(rest) = line.strip_prefix("TX:") {
            ("TX", rest)
        } else if let Some(rest) = line.strip_prefix('>') {
            ("TX", rest)
        } else {
            ("RX", line)
        };
        if dir == "TX" {
            continue;
        }
        let mut bytes = Vec::new();
        for tok in hex_part.split_whitespace() {
            let b = u8::from_str_radix(tok, 16)
                .map_err(|e| format!("line {}: bad hex '{tok}': {e}", lineno + 1))?;
            bytes.push(b);
        }
        if !bytes.is_empty() {
            chunks.push(bytes);
        }
    }
    Ok(chunks)
}

pub struct ReplayTransport {
    name: String,
    open: bool,
    /// RX chunks waiting for the next ENQ (queue).
    pending_rx: VecDeque<Vec<u8>>,
    /// Bytes currently available to read.
    rx: VecDeque<u8>,
}

impl ReplayTransport {
    pub fn from_hex_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path.as_ref()).map_err(|e| e.to_string())?;
        Self::from_hex_str(&text, path.as_ref().display().to_string())
    }

    pub fn from_hex_str(text: &str, name: impl Into<String>) -> Result<Self, String> {
        let chunks = parse_hex_capture(text)?;
        Ok(Self {
            name: name.into(),
            open: false,
            pending_rx: VecDeque::from(chunks),
            rx: VecDeque::new(),
        })
    }

    pub fn from_rx_chunks(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            name: "replay".into(),
            open: false,
            pending_rx: VecDeque::from(chunks),
            rx: VecDeque::new(),
        }
    }
}

impl Transport for ReplayTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Simulator
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn open(&mut self) -> io::Result<()> {
        self.open = true;
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.open = false;
        self.rx.clear();
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        if !self.open {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "replay closed"));
        }
        for &b in data {
            match b {
                ENQ => {
                    if let Some(chunk) = self.pending_rx.pop_front() {
                        self.rx.extend(chunk);
                    } else {
                        self.rx.push_back(NAK);
                    }
                }
                ACK | NAK => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<usize> {
        if !self.open {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "replay closed"));
        }
        if self.rx.is_empty() {
            std::thread::sleep(timeout.min(Duration::from_millis(1)));
            return Ok(0);
        }
        let n = buf.len().min(self.rx.len());
        for i in 0..n {
            buf[i] = self.rx.pop_front().unwrap();
        }
        Ok(n)
    }
}
