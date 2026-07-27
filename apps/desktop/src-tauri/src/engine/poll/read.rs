//! ENQ write + timed read from the active transport.

use crate::connection::shot_latency::PollReadResultKind;
use crate::protocol::encode_enq;
use crate::transport::{ConnectionStatus, Transport};
use super::super::{emit_conn, ConnectionUpdate, StandEngine};
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Write ENQ. Returns `false` if the write failed (caller should break the poll loop).
pub(super) fn write_enq(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    transport: &mut dyn Transport,
) -> bool {
    if let Err(e) = transport.write_all(&encode_enq()) {
        emit_conn(
            app,
            engine,
            generation,
            ConnectionUpdate {
                status: ConnectionStatus::Disconnected,
                transport: transport.kind(),
                port: Some(transport.name().to_string()),
                detail: Some(e.to_string()),
            },
        );
        return false;
    }
    true
}

/// Optional diagnose-only timestamps around one poll read.
#[derive(Debug, Clone, Copy)]
pub(super) struct ReadTiming {
    pub started: Instant,
    pub returned: Instant,
}

impl ReadTiming {
    pub(super) fn duration_ms(self) -> u64 {
        self.returned
            .saturating_duration_since(self.started)
            .as_millis() as u64
    }
}

/// Product `n` plus optional timing (only when caller requested diagnose timing).
pub(super) struct ReadChunkOutcome {
    /// `None` = transport error (caller should break).
    pub n: Option<usize>,
    pub timing: Option<ReadTiming>,
}

/// Classify Bridge/sim `Ok(n)` for diagnose JSONL (`Err` never reaches here).
pub(super) fn classify_read_result(n: usize) -> PollReadResultKind {
    if n > 0 {
        PollReadResultKind::Bytes
    } else {
        PollReadResultKind::Empty
    }
}

/// Read into `buf`.
///
/// - Always: `read_timeout(50 ms)`, same Ok/Err handling as pre-v2.
/// - `want_timing`: when false, no `Instant::now` and `timing` is `None`.
///   When true, stamps start/end around the same read (caller already knows diag is on).
pub(super) fn read_chunk(
    app: &AppHandle,
    engine: &StandEngine,
    generation: u64,
    transport: &mut dyn Transport,
    buf: &mut [u8; 256],
    want_timing: bool,
) -> ReadChunkOutcome {
    let started = if want_timing {
        Some(Instant::now())
    } else {
        None
    };
    let n = match transport.read_timeout(buf, Duration::from_millis(50)) {
        Ok(n) => Some(n),
        Err(e) => {
            emit_conn(
                app,
                engine,
                generation,
                ConnectionUpdate {
                    status: ConnectionStatus::Disconnected,
                    transport: transport.kind(),
                    port: Some(transport.name().to_string()),
                    detail: Some(e.to_string()),
                },
            );
            None
        }
    };
    let timing = started.map(|started| ReadTiming {
        started,
        returned: Instant::now(),
    });
    ReadChunkOutcome { n, timing }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure contract: timing helper only when requested — no sleep.
    #[test]
    fn read_timing_only_when_requested() {
        fn stamp(want: bool) -> Option<ReadTiming> {
            let started = if want {
                Some(Instant::now())
            } else {
                None
            };
            started.map(|started| ReadTiming {
                started,
                returned: started, // zero-duration stand-in; no sleep
            })
        }
        assert!(stamp(false).is_none());
        let on = stamp(true).expect("timing on");
        assert_eq!(on.duration_ms(), 0);
        assert_eq!(classify_read_result(0), PollReadResultKind::Empty);
        assert_eq!(classify_read_result(7), PollReadResultKind::Bytes);
    }

    #[test]
    fn outcome_shape_matches_pre_v2_n_semantics() {
        // Product success / empty / error are carried only in `n`; timing is orthogonal.
        let off = ReadChunkOutcome {
            n: Some(0),
            timing: None,
        };
        assert_eq!(off.n, Some(0));
        assert!(off.timing.is_none());
        let on = ReadChunkOutcome {
            n: Some(12),
            timing: Some(ReadTiming {
                started: Instant::now(),
                returned: Instant::now(),
            }),
        };
        assert_eq!(on.n, Some(12));
        assert!(on.timing.is_some());
        let err = ReadChunkOutcome {
            n: None,
            timing: None,
        };
        assert!(err.n.is_none());
    }
}
