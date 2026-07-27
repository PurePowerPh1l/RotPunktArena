//! Session sink fanout: coherent enabled+epoch state and epoch-tagged chunks.
//!
//! Owner is the sole writer; Poll/Bridge consumes and drops stale epochs.

use crate::protocol::{Incoming, RedDotStreamParser};

/// Coherent sink registration state (Owner-only; never split-read).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SinkFanout {
    pub enabled: bool,
    pub epoch: u64,
}

impl Default for SinkFanout {
    fn default() -> Self {
        Self {
            enabled: false,
            epoch: 0,
        }
    }
}

/// One RX slice tagged with the fanout epoch at enqueue time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SinkChunk {
    pub epoch: u64,
    pub bytes: Vec<u8>,
    /// DIAGNOSE-ONLY latency provenance. Never used for fanout/epoch decisions.
    pub diag: Option<super::shot_latency::SinkChunkDiag>,
}

/// Result of applying inbound link bytes under a fanout snapshot.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FanoutApply {
    /// Active series: enqueue this chunk (epoch from the same snapshot).
    Enqueue(SinkChunk),
    /// Pause: complete shot frames that must be ACKed then discarded.
    PauseAck { complete_shots: u32 },
    /// Nothing to enqueue or ACK (NeedMore / Skip / empty).
    Idle,
}

/// Pure fanout decision used by `pump_linked` (and C2 unit tests).
///
/// `fanout` must be a single snapshot for the whole call — no re-read of enabled/epoch.
pub(crate) fn apply_fanout_bytes(
    fanout: SinkFanout,
    pause_parser: &mut RedDotStreamParser,
    bytes: &[u8],
) -> FanoutApply {
    if bytes.is_empty() {
        return FanoutApply::Idle;
    }
    if fanout.enabled {
        return FanoutApply::Enqueue(SinkChunk {
            epoch: fanout.epoch,
            bytes: bytes.to_vec(),
            diag: None,
        });
    }
    let mut complete_shots = 0u32;
    for msg in pause_parser.push(bytes) {
        if matches!(msg, Incoming::ShotFrame(_)) {
            complete_shots += 1;
        }
    }
    if complete_shots > 0 {
        FanoutApply::PauseAck { complete_shots }
    } else {
        FanoutApply::Idle
    }
}

/// Consumer filter: stale epochs never reach the poll parser.
#[inline]
pub(crate) fn chunk_bytes_for_poll(registered_epoch: u64, chunk: &SinkChunk) -> Option<&[u8]> {
    if chunk.epoch == registered_epoch {
        Some(chunk.bytes.as_slice())
    } else {
        None
    }
}
