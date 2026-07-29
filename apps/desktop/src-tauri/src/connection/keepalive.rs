//! Linked-state ENQ keepalive + read fanout to session sink.

use super::connect_policy::ConnectPhase;
use super::event::ConnectionEventKind;
use super::owner::Owner;
use super::shot_latency::{self, SinkChunkDiag};
use super::sink::{apply_fanout_bytes, FanoutApply};
use super::status::ConnectionStatus;
use super::timing::{ENQ_INTERVAL, ENQ_WRITE_TIMEOUT, IO_FAIL_LIMIT, READ_SLICE};
use crate::protocol::{encode_ack, encode_enq};
use crate::transport::rfcomm::error::TransportError;
use std::sync::mpsc::TrySendError;
use std::time::{Duration, Instant};

/// Bounded backpressure when the session sink is full: retry instead of
/// silently dropping RX bytes (the poll worker usually drains within ms;
/// longer stalls come from snapshot I/O). Window stays well below the ENQ
/// interval so the keepalive cadence survives.
const SINK_FULL_RETRY_WINDOW: Duration = Duration::from_millis(500);
const SINK_FULL_RETRY_SLICE: Duration = Duration::from_millis(5);

impl Owner {
    pub(crate) fn pump_linked(&mut self) {
        let now = Instant::now();
        if now >= self.next_enq {
            self.next_enq = now + ENQ_INTERVAL;
            if let Some(sock) = self.socket.as_mut() {
                if let Err(e) = sock.write_all(&encode_enq(), ENQ_WRITE_TIMEOUT) {
                    self.io_fail_streak += 1;
                    self.diag(
                        "enq_fail",
                        ConnectionStatus::Linked,
                        &format!("{e} streak={}", self.io_fail_streak),
                        None,
                        None,
                    );
                    if self.io_fail_streak >= IO_FAIL_LIMIT {
                        self.on_link_lost(&format!("ENQ write: {e}"));
                    }
                    return;
                }
                self.io_fail_streak = 0;
                // DIAGNOSE-ONLY — does not change ENQ timing or product path.
                if shot_latency::is_enabled() {
                    self.diag_last_enq_sent_at = Some(Instant::now());
                }
            }
        }

        let mut buf = [0u8; 256];
        let n = match self.socket.as_mut() {
            Some(sock) => match sock.read(&mut buf, READ_SLICE) {
                Ok(n) => {
                    self.io_fail_streak = 0;
                    n
                }
                Err(TransportError::Timeout) => 0,
                Err(e) => {
                    self.io_fail_streak += 1;
                    self.diag(
                        "read_fail",
                        ConnectionStatus::Linked,
                        &format!("{e} streak={}", self.io_fail_streak),
                        None,
                        None,
                    );
                    if self.io_fail_streak >= IO_FAIL_LIMIT {
                        self.on_link_lost(&format!("read: {e}"));
                    }
                    return;
                }
            },
            None => return,
        };
        if n == 0 {
            return;
        }

        // DIAGNOSE-ONLY provenance for this RX slice (only when diag enabled).
        let fanout = self.sink_fanout;
        match apply_fanout_bytes(fanout, &mut self.parser, &buf[..n]) {
            FanoutApply::Enqueue(mut chunk) => {
                if shot_latency::is_enabled() {
                    self.diag_rx_seq = self.diag_rx_seq.wrapping_add(1);
                    let owner_rx_at = Instant::now();
                    chunk.diag = Some(SinkChunkDiag {
                        rx_seq: self.diag_rx_seq,
                        owner_rx_at,
                        last_enq_sent_at: self.diag_last_enq_sent_at,
                        sink_enqueued_at: Instant::now(),
                    });
                }
                match self.sink_tx.try_send(chunk) {
                    Ok(()) => shot_latency::record_try_send_ok(),
                    Err(TrySendError::Full(returned)) => {
                        shot_latency::record_try_send_full();
                        self.retry_sink_send(returned);
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        shot_latency::record_try_send_disconnected()
                    }
                }
            }
            FanoutApply::PauseAck { complete_shots } => {
                // Same write path as session ACK (WriteBytes uses 500ms).
                for _ in 0..complete_shots {
                    if let Some(sock) = self.socket.as_mut() {
                        if let Err(e) =
                            sock.write_all(&encode_ack(), Duration::from_millis(500))
                        {
                            self.on_link_lost(&format!("pause ACK: {e}"));
                            return;
                        }
                    }
                }
            }
            FanoutApply::Idle => {}
        }
    }

    /// Bounded retry after a full sink; drops (with diag) only when the
    /// window elapses so RX bytes are not lost silently on transient stalls.
    fn retry_sink_send(&mut self, chunk: super::sink::SinkChunk) {
        let deadline = Instant::now() + SINK_FULL_RETRY_WINDOW;
        let mut pending = chunk;
        loop {
            std::thread::sleep(SINK_FULL_RETRY_SLICE);
            match self.sink_tx.try_send(pending) {
                Ok(()) => {
                    shot_latency::record_try_send_ok();
                    return;
                }
                Err(TrySendError::Full(returned)) => {
                    shot_latency::record_try_send_full();
                    if Instant::now() >= deadline {
                        self.diag(
                            "sink_full_drop",
                            ConnectionStatus::Linked,
                            "RX-Chunk verworfen — Session-Sink dauerhaft voll",
                            None,
                            None,
                        );
                        return;
                    }
                    pending = returned;
                }
                Err(TrySendError::Disconnected(_)) => {
                    shot_latency::record_try_send_disconnected();
                    return;
                }
            }
        }
    }

    pub(crate) fn on_link_lost(&mut self, reason: &str) {
        self.socket = None;
        self.io_fail_streak = 0;
        self.diag(
            "link_lost",
            ConnectionStatus::Idle,
            reason,
            None,
            None,
        );
        self.emit(
            ConnectionEventKind::LinkInterrupted,
            ConnectionStatus::Idle,
            reason,
        );
        self.set_connect_phase(ConnectPhase::Idle);
        self.set_status(
            ConnectionStatus::Idle,
            &format!("{reason} — Nicht verbunden"),
        );
    }
}
