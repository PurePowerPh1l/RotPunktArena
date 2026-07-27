//! Session bridge: RegisterSink on open; never closes the app-lifetime socket.

use super::command::ConnectionCommand;
use super::handle::ConnectionHandle;
use super::shot_latency::{self, SinkVisit};
use super::sink::chunk_bytes_for_poll;
use super::status::ConnectionStatus;
use crate::protocol::encode_enq;
use std::thread;
use std::time::{Duration, Instant};

/// Session-facing transport: RegisterSink on open, UnregisterSink on close; never closes socket.
pub struct RfcommBridgeTransport {
    handle: ConnectionHandle,
    name: String,
    open: bool,
    /// Epoch captured after Owner completes RegisterSink for this series.
    registered_epoch: u64,
}

impl RfcommBridgeTransport {
    pub fn new(handle: ConnectionHandle) -> Self {
        let name = handle
            .target()
            .map(|t| {
                if let Some(com) = &t.com_port {
                    format!("{com} ({})", t.display_name)
                } else {
                    format!("RFCOMM {}", t.addr_hex())
                }
            })
            .unwrap_or_else(|| "BT-Link".into());
        Self {
            handle,
            name,
            open: false,
            registered_epoch: 0,
        }
    }

    fn wait_registered_epoch(&mut self) -> std::io::Result<()> {
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            if self.handle.sink_registered() {
                self.registered_epoch = self.handle.sink_epoch();
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::other(
                    "RegisterSink Timeout — Owner hat Sink nicht freigegeben",
                ));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

impl crate::transport::Transport for RfcommBridgeTransport {
    fn kind(&self) -> crate::transport::TransportKind {
        crate::transport::TransportKind::Rfcomm
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn open(&mut self) -> std::io::Result<()> {
        if self.handle.status() != ConnectionStatus::Linked {
            return Err(std::io::Error::other(format!(
                "Kein aktiver Bluetooth-Link ({}, {}) — zuerst Badge „Verbunden“ abwarten oder Debug „Ziel verbinden“",
                self.handle.status().as_str(),
                self.handle.last_reason()
            )));
        }
        self.handle
            .send(ConnectionCommand::RegisterSink)
            .map_err(std::io::Error::other)?;
        self.wait_registered_epoch()?;
        self.open = true;
        Ok(())
    }

    fn close(&mut self) -> std::io::Result<()> {
        let _ = self.handle.send(ConnectionCommand::UnregisterSink);
        self.open = false;
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        if data == encode_enq().as_slice() {
            return Ok(());
        }
        self.handle
            .send(ConnectionCommand::WriteBytes(data.to_vec()))
            .map_err(std::io::Error::other)
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> std::io::Result<usize> {
        let deadline = Instant::now() + timeout;
        let registered = self.registered_epoch;
        loop {
            while let Some(chunk) = self.handle.try_recv_sink_chunk() {
                shot_latency::record_bridge_try_recv();
                // Stale epoch: drop without feeding poll parser (no parser reset here).
                if let Some(bytes) = chunk_bytes_for_poll(registered, &chunk) {
                    let n = bytes.len().min(buf.len());
                    buf[..n].copy_from_slice(&bytes[..n]);
                    // DIAGNOSE-ONLY: expose provenance to poll via thread-local.
                    if let Some(diag) = chunk.diag {
                        shot_latency::note_bridge_visit(SinkVisit {
                            diag,
                            bridge_received_at: Instant::now(),
                        });
                    }
                    return Ok(n);
                }
                shot_latency::record_stale_epoch_drop();
            }
            if Instant::now() >= deadline {
                return Ok(0);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
