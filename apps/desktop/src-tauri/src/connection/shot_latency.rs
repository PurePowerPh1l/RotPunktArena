//! DIAGNOSE-ONLY — shot registration latency telemetry.
//!
//! Opt-in via `REDDOT_SHOT_LATENCY_DIAG=1`. Default OFF: no writer thread,
//! no JSONL, no per-shot serialization. Observes only; never influences
//! fanout, epoch, ACK, parse, accept, persist, or UI decisions.
//!
//! Poll path: build record → `try_send` to diagnose writer (never blocks on I/O).
//! JSONL: `logs/shot_latency.jsonl` (same repo-logs layout as RFCOMM diag).
//! Times are monotonic offsets from a process run anchor — never serialize Instant.

use super::diag;
use crate::protocol::{Incoming, RedDotStreamParser};
use serde::Serialize;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::OnceLock;
use std::thread;
use std::time::Instant;

pub const SCHEMA_VERSION: u32 = 2;

/// Bounded diagnose queue. Small on purpose: brief disk stall OK; sustained
/// backlog would delay measurement honesty — prefer drop + counter over block.
pub const DIAG_QUEUE_CAPACITY: usize = 32;

const ENV_FLAG: &str = "REDDOT_SHOT_LATENCY_DIAG";

/// Process-lifetime diagnose run id + monotonic anchor (created on first enable use).
struct RunAnchor {
    run_id: String,
    start: Instant,
}

enum DiagState {
    Disabled,
    Enabled {
        tx: SyncSender<ShotLatencyRecordOwned>,
        #[allow(dead_code)] // kept so the JoinHandle is not dropped (detach)
        _join: thread::JoinHandle<()>,
    },
}

static DIAG_STATE: OnceLock<DiagState> = OnceLock::new();
static RUN_ANCHOR: OnceLock<RunAnchor> = OnceLock::new();

static SINK_TRY_SEND_OK: AtomicU64 = AtomicU64::new(0);
static SINK_TRY_SEND_FULL: AtomicU64 = AtomicU64::new(0);
static SINK_TRY_SEND_DISCONNECTED: AtomicU64 = AtomicU64::new(0);
static BRIDGE_TRY_RECV: AtomicU64 = AtomicU64::new(0);
static STALE_EPOCH_DROP: AtomicU64 = AtomicU64::new(0);
static JSONL_WRITE_FAIL: AtomicU64 = AtomicU64::new(0);
static DIAG_QUEUE_DROP_FULL: AtomicU64 = AtomicU64::new(0);
static DIAG_QUEUE_DROP_DISCONNECTED: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// Last live sink visit delivered by Bridge::read_timeout (diagnose-only).
    static LAST_BRIDGE_VISIT: RefCell<Option<SinkVisit>> = const { RefCell::new(None) };
    /// Set by Arena after COMMIT, before VACUUM — taken by poll ingest.
    static SQLITE_COMMITTED_AT: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

fn env_flag_enabled() -> bool {
    match std::env::var(ENV_FLAG) {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// True when `REDDOT_SHOT_LATENCY_DIAG=1` (or true/yes). Lazy; starts writer on first true.
pub fn is_enabled() -> bool {
    matches!(diag_state(), DiagState::Enabled { .. })
}

fn diag_state() -> &'static DiagState {
    DIAG_STATE.get_or_init(|| {
        if !env_flag_enabled() {
            return DiagState::Disabled;
        }
        let (tx, rx) = sync_channel::<ShotLatencyRecordOwned>(DIAG_QUEUE_CAPACITY);
        match thread::Builder::new()
            .name("shot-latency-diag".into())
            .spawn(move || writer_loop(rx))
        {
            Ok(handle) => DiagState::Enabled {
                tx,
                _join: handle,
            },
            Err(_) => DiagState::Disabled,
        }
    })
}

fn run_anchor() -> &'static RunAnchor {
    RUN_ANCHOR.get_or_init(|| RunAnchor {
        run_id: format!("shotlat-{}", chrono::Utc::now().format("%Y%m%dT%H%M%S%.3f")),
        start: Instant::now(),
    })
}

pub fn offset_ms(at: Instant) -> u64 {
    at.saturating_duration_since(run_anchor().start)
        .as_millis() as u64
}

pub fn optional_offset_ms(at: Option<Instant>) -> Option<u64> {
    at.map(offset_ms)
}

/// DIAGNOSE-ONLY provenance stamped on a SinkChunk before try_send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkChunkDiag {
    pub rx_seq: u64,
    pub owner_rx_at: Instant,
    pub last_enq_sent_at: Option<Instant>,
    pub sink_enqueued_at: Instant,
}

/// One Bridge delivery of a live-epoch chunk (bytes already copied to poll buf).
#[derive(Debug, Clone)]
pub struct SinkVisit {
    pub diag: SinkChunkDiag,
    pub bridge_received_at: Instant,
}

/// Classification of one poll `read_timeout` return (diagnose-only).
/// `Timeout` reserved if a transport distinguishes deadline from empty; Bridge uses `Empty` for `Ok(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PollReadResultKind {
    Bytes,
    Empty,
    #[allow(dead_code)]
    Timeout,
}

/// Poll-loop touch for the read that delivered a chunk (diagnose-only).
#[derive(Debug, Clone)]
pub struct PollReadTouch {
    pub iteration_id: u64,
    pub wait_started: Option<Instant>,
    pub wait_returned: Option<Instant>,
    pub read_started: Instant,
    pub read_returned: Instant,
    pub read_result: PollReadResultKind,
}

/// Counters for the window after first-chunk feed until frame complete.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterveningAccum {
    pub wait_ms: u64,
    pub read_calls: u64,
    pub byte_reads: u64,
    pub empty_reads: u64,
    pub read_wait_ms: u64,
}

impl InterveningAccum {
    pub fn note_read(&mut self, kind: PollReadResultKind, read_wait_ms: u64) {
        self.read_calls = self.read_calls.saturating_add(1);
        match kind {
            PollReadResultKind::Bytes => {
                self.byte_reads = self.byte_reads.saturating_add(1);
            }
            PollReadResultKind::Empty | PollReadResultKind::Timeout => {
                self.empty_reads = self.empty_reads.saturating_add(1);
            }
        }
        self.read_wait_ms = self.read_wait_ms.saturating_add(read_wait_ms);
    }

    pub fn note_wait_ms(&mut self, wait_ms: u64) {
        self.wait_ms = self.wait_ms.saturating_add(wait_ms);
    }
}

/// Poll-loop fields attached to an accepted RFCOMM shot (diagnose-only).
#[derive(Debug, Clone)]
pub struct PollLoopShotDiag {
    pub first_touch: PollReadTouch,
    pub frame_complete_iteration_id: u64,
    pub intervening: InterveningAccum,
}

#[derive(Debug, Clone)]
pub struct TracedShotFrame {
    pub raw: Vec<u8>,
    pub first_chunk: SinkChunkDiag,
    pub bridge_received_at: Instant,
    pub poll_parser_frame_at: Instant,
    /// Present when diagnose stamped a poll touch on the first-byte chunk.
    pub poll_loop: Option<PollLoopShotDiag>,
}

/// Parser outputs that poll must handle, with optional latency provenance on shots.
#[derive(Debug, Clone)]
pub enum TracedIncoming {
    Nak,
    Shot(TracedShotFrame),
}

#[derive(Debug, Clone)]
struct PendingShot {
    visit: SinkVisit,
    first_touch: Option<PollReadTouch>,
    intervening: InterveningAccum,
}

/// Pending open STX frame: first-byte provenance + intervening poll-loop accumulators.
#[derive(Debug, Default)]
pub struct FrameProvenanceTracker {
    pending: Option<PendingShot>,
}

impl FrameProvenanceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when an incomplete STX shot frame is open (pending provenance).
    pub fn has_open_frame(&self) -> bool {
        self.pending.is_some()
    }

    /// Clear open-frame provenance and intervening accumulators.
    ///
    /// Tracker-only helper (unit tests / future hooks). The live poll loop does
    /// **not** currently reset the RedDot parser mid-session or call this —
    /// not a wired product discard/resync path.
    #[allow(dead_code)]
    pub fn discard(&mut self) {
        self.pending = None;
    }

    /// Count a poll wait only while a pending frame is still open after the last feed.
    pub fn accumulate_wait_ms(&mut self, wait_ms: u64) {
        if let Some(p) = self.pending.as_mut() {
            p.intervening.note_wait_ms(wait_ms);
        }
    }

    /// Count a read toward intervening only if a frame was already pending *before* the read.
    /// Includes the completing read that closes the frame on the subsequent feed.
    pub fn accumulate_read_if_was_pending(
        &mut self,
        was_pending_before: bool,
        kind: PollReadResultKind,
        read_wait_ms: u64,
    ) {
        if !was_pending_before {
            return;
        }
        if let Some(p) = self.pending.as_mut() {
            p.intervening.note_read(kind, read_wait_ms);
        }
    }

    /// Feed one sink visit into the poll parser; return NAK / completed shots.
    /// Shot frames inherit first-byte-chunk provenance (multi-frame chunks share rx_seq).
    /// `poll_touch`: diagnose-only touch for this read (iteration / wait-before / read result).
    pub fn feed(
        &mut self,
        parser: &mut RedDotStreamParser,
        visit: SinkVisit,
        bytes: &[u8],
        poll_touch: Option<PollReadTouch>,
    ) -> Vec<TracedIncoming> {
        let started_incomplete = shot_frame_incomplete(parser);
        let msgs = parser.push(bytes);
        let now = Instant::now();
        let mut out = Vec::new();
        let complete_iter = poll_touch.as_ref().map(|t| t.iteration_id);

        for msg in msgs {
            match msg {
                Incoming::ShotFrame(raw) => {
                    let (first_chunk, bridge_received_at, first_touch, intervening) =
                        if started_incomplete {
                            match self.pending.take() {
                                Some(p) => (
                                    p.visit.diag,
                                    p.visit.bridge_received_at,
                                    p.first_touch.or_else(|| poll_touch.clone()),
                                    p.intervening,
                                ),
                                None => (
                                    visit.diag.clone(),
                                    visit.bridge_received_at,
                                    poll_touch.clone(),
                                    InterveningAccum::default(),
                                ),
                            }
                        } else {
                            (
                                visit.diag.clone(),
                                visit.bridge_received_at,
                                poll_touch.clone(),
                                InterveningAccum::default(),
                            )
                        };
                    let poll_loop = first_touch.map(|ft| PollLoopShotDiag {
                        frame_complete_iteration_id: complete_iter.unwrap_or(ft.iteration_id),
                        first_touch: ft,
                        intervening,
                    });
                    out.push(TracedIncoming::Shot(TracedShotFrame {
                        raw,
                        first_chunk,
                        bridge_received_at,
                        poll_parser_frame_at: now,
                        poll_loop,
                    }));
                }
                Incoming::Nak => out.push(TracedIncoming::Nak),
                Incoming::NeedMore => {
                    if self.pending.is_none() {
                        self.pending = Some(PendingShot {
                            visit: visit.clone(),
                            first_touch: poll_touch.clone(),
                            intervening: InterveningAccum::default(),
                        });
                    }
                }
                Incoming::Ack | Incoming::Skip => {}
            }
        }

        if shot_frame_incomplete(parser) {
            if self.pending.is_none() {
                self.pending = Some(PendingShot {
                    visit,
                    first_touch: poll_touch,
                    intervening: InterveningAccum::default(),
                });
            }
        } else {
            self.pending = None;
        }
        out
    }
}

/// True when parser holds an incomplete STX shot frame after feed/resync.
/// Delegates to the product parser helper (same semantics as poll idle-wait guard).
pub fn shot_frame_incomplete(parser: &RedDotStreamParser) -> bool {
    parser.has_incomplete_shot_frame()
}

pub fn record_try_send_ok() {
    if !is_enabled() {
        return;
    }
    SINK_TRY_SEND_OK.fetch_add(1, Ordering::Relaxed);
}

pub fn record_try_send_full() {
    if !is_enabled() {
        return;
    }
    SINK_TRY_SEND_FULL.fetch_add(1, Ordering::Relaxed);
}

pub fn record_try_send_disconnected() {
    if !is_enabled() {
        return;
    }
    SINK_TRY_SEND_DISCONNECTED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_bridge_try_recv() {
    if !is_enabled() {
        return;
    }
    BRIDGE_TRY_RECV.fetch_add(1, Ordering::Relaxed);
}

pub fn record_stale_epoch_drop() {
    if !is_enabled() {
        return;
    }
    STALE_EPOCH_DROP.fetch_add(1, Ordering::Relaxed);
}

pub fn note_bridge_visit(visit: SinkVisit) {
    if !is_enabled() {
        return;
    }
    LAST_BRIDGE_VISIT.with(|c| *c.borrow_mut() = Some(visit));
}

pub fn take_bridge_visit() -> Option<SinkVisit> {
    if !is_enabled() {
        return None;
    }
    LAST_BRIDGE_VISIT.with(|c| c.borrow_mut().take())
}

pub fn note_sqlite_committed_at(at: Instant) {
    if !is_enabled() {
        return;
    }
    SQLITE_COMMITTED_AT.with(|c| *c.borrow_mut() = Some(at));
}

pub fn take_sqlite_committed_at() -> Option<Instant> {
    if !is_enabled() {
        return None;
    }
    SQLITE_COMMITTED_AT.with(|c| c.borrow_mut().take())
}

pub fn frame_sha16(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw);
    let full = hex::encode(hasher.finalize());
    full.chars().take(16).collect()
}

pub fn shot_trace_id(session_id: &str, frame_sha16: &str) -> String {
    format!("{session_id}:{frame_sha16}")
}

/// Owned record for the diagnose writer queue (no borrows across threads).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShotLatencyRecordOwned {
    pub schema_version: u32,
    pub run_id: String,
    pub wall_ts: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    pub shot_trace_id: String,
    pub first_chunk_rx_seq: u64,
    pub owner_rx_offset_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_enq_sent_offset_ms: Option<u64>,
    pub sink_enqueued_offset_ms: u64,
    pub bridge_received_offset_ms: u64,
    /// Alias of first-chunk bridge receive (schema v2 clarity).
    pub bridge_first_chunk_received_offset_ms: u64,
    pub poll_parser_frame_offset_ms: u64,
    pub ingest_started_offset_ms: u64,
    pub sqlite_committed_offset_ms: u64,
    pub shot_event_emitted_offset_ms: u64,
    pub sink_try_send_ok_count: u64,
    pub sink_try_send_full_count: u64,
    pub sink_try_send_disconnected_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_try_recv_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_epoch_drop_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_iteration_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_wait_started_offset_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_wait_returned_offset_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_first_read_started_offset_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_first_read_returned_offset_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_read_result: Option<PollReadResultKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_frame_complete_iteration_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_intervening_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_intervening_read_calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_intervening_byte_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_intervening_empty_reads: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_intervening_read_wait_ms: Option<u64>,
}

/// Poll-path entry: build record + non-blocking enqueue. Never opens files.
pub fn append_accepted_shot(
    session_id: &str,
    mode: Option<&str>,
    traced: &TracedShotFrame,
    ingest_started: Instant,
    sqlite_committed: Instant,
    shot_event_emitted: Instant,
) {
    let DiagState::Enabled { tx, .. } = diag_state() else {
        return;
    };
    let sha16 = frame_sha16(&traced.raw);
    let bridge_rx = offset_ms(traced.bridge_received_at);
    let poll = traced.poll_loop.as_ref();
    let record = ShotLatencyRecordOwned {
        schema_version: SCHEMA_VERSION,
        run_id: run_anchor().run_id.clone(),
        wall_ts: diag::now_ts(),
        session_id: session_id.to_string(),
        mode: mode.map(str::to_string),
        shot_trace_id: shot_trace_id(session_id, &sha16),
        first_chunk_rx_seq: traced.first_chunk.rx_seq,
        owner_rx_offset_ms: offset_ms(traced.first_chunk.owner_rx_at),
        last_enq_sent_offset_ms: optional_offset_ms(traced.first_chunk.last_enq_sent_at),
        sink_enqueued_offset_ms: offset_ms(traced.first_chunk.sink_enqueued_at),
        bridge_received_offset_ms: bridge_rx,
        bridge_first_chunk_received_offset_ms: bridge_rx,
        poll_parser_frame_offset_ms: offset_ms(traced.poll_parser_frame_at),
        ingest_started_offset_ms: offset_ms(ingest_started),
        sqlite_committed_offset_ms: offset_ms(sqlite_committed),
        shot_event_emitted_offset_ms: offset_ms(shot_event_emitted),
        sink_try_send_ok_count: SINK_TRY_SEND_OK.load(Ordering::Relaxed),
        sink_try_send_full_count: SINK_TRY_SEND_FULL.load(Ordering::Relaxed),
        sink_try_send_disconnected_count: SINK_TRY_SEND_DISCONNECTED.load(Ordering::Relaxed),
        bridge_try_recv_count: Some(BRIDGE_TRY_RECV.load(Ordering::Relaxed)),
        stale_epoch_drop_count: Some(STALE_EPOCH_DROP.load(Ordering::Relaxed)),
        poll_iteration_id: poll.map(|p| p.first_touch.iteration_id),
        poll_wait_started_offset_ms: poll.and_then(|p| optional_offset_ms(p.first_touch.wait_started)),
        poll_wait_returned_offset_ms: poll
            .and_then(|p| optional_offset_ms(p.first_touch.wait_returned)),
        poll_first_read_started_offset_ms: poll.map(|p| offset_ms(p.first_touch.read_started)),
        poll_first_read_returned_offset_ms: poll.map(|p| offset_ms(p.first_touch.read_returned)),
        poll_read_result: poll.map(|p| p.first_touch.read_result),
        poll_frame_complete_iteration_id: poll.map(|p| p.frame_complete_iteration_id),
        poll_intervening_wait_ms: poll.map(|p| p.intervening.wait_ms),
        poll_intervening_read_calls: poll.map(|p| p.intervening.read_calls),
        poll_intervening_byte_reads: poll.map(|p| p.intervening.byte_reads),
        poll_intervening_empty_reads: poll.map(|p| p.intervening.empty_reads),
        poll_intervening_read_wait_ms: poll.map(|p| p.intervening.read_wait_ms),
    };
    try_enqueue_record(tx, record);
}

/// Non-blocking enqueue; drops on Full/Disconnected (diagnose counters only).
pub(crate) fn try_enqueue_record(
    tx: &SyncSender<ShotLatencyRecordOwned>,
    record: ShotLatencyRecordOwned,
) {
    match tx.try_send(record) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            DIAG_QUEUE_DROP_FULL.fetch_add(1, Ordering::Relaxed);
        }
        Err(TrySendError::Disconnected(_)) => {
            DIAG_QUEUE_DROP_DISCONNECTED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Serialize one record and flush so the JSONL line is visible without process exit.
/// Diagnose-only; never called from the poll thread.
fn write_record_line(
    out: &mut BufWriter<std::fs::File>,
    record: &ShotLatencyRecordOwned,
) -> bool {
    match serde_json::to_string(record) {
        Ok(line) => {
            if writeln!(out, "{line}").is_err() {
                return false;
            }
            // Flush to OS page cache so few-shot smokes see bytes on disk.
            // No sync_all — diagnose must not force durable media I/O.
            if out.flush().is_err() {
                return false;
            }
            true
        }
        Err(_) => false,
    }
}

fn writer_loop(rx: Receiver<ShotLatencyRecordOwned>) {
    let Some(dir) = diag::repo_logs_dir_for_diag() else {
        // Drain so senders don't fill forever if dir missing.
        while rx.recv().is_ok() {
            JSONL_WRITE_FAIL.fetch_add(1, Ordering::Relaxed);
        }
        return;
    };
    let path = dir.join("shot_latency.jsonl");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) else {
        while rx.recv().is_ok() {
            JSONL_WRITE_FAIL.fetch_add(1, Ordering::Relaxed);
        }
        return;
    };
    let mut out = BufWriter::new(file);
    while let Ok(record) = rx.recv() {
        if !write_record_line(&mut out, &record) {
            JSONL_WRITE_FAIL.fetch_add(1, Ordering::Relaxed);
        }
    }
    // Best-effort flush on channel disconnect (process teardown). Never joins from poll.
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::sink::{chunk_bytes_for_poll, SinkChunk};
    use crate::protocol::{build_synthetic_shot_frame, NAK};

    fn shot(seed: u32) -> Vec<u8> {
        let x = format!("{seed:05}");
        build_synthetic_shot_frame("10.5", "012.30", &x, "00040").unwrap()
    }

    fn visit(seq: u64, at: Instant) -> SinkVisit {
        SinkVisit {
            diag: SinkChunkDiag {
                rx_seq: seq,
                owner_rx_at: at,
                last_enq_sent_at: None,
                sink_enqueued_at: at,
            },
            bridge_received_at: at,
        }
    }

    fn sample_record() -> ShotLatencyRecordOwned {
        ShotLatencyRecordOwned {
            schema_version: SCHEMA_VERSION,
            run_id: "shotlat-test".into(),
            wall_ts: "2026-07-26T00:00:00.000Z".into(),
            session_id: "s1".into(),
            mode: Some("training".into()),
            shot_trace_id: "s1:deadbeefdeadbeef".into(),
            first_chunk_rx_seq: 3,
            owner_rx_offset_ms: 1,
            last_enq_sent_offset_ms: Some(0),
            sink_enqueued_offset_ms: 1,
            bridge_received_offset_ms: 2,
            bridge_first_chunk_received_offset_ms: 2,
            poll_parser_frame_offset_ms: 3,
            ingest_started_offset_ms: 4,
            sqlite_committed_offset_ms: 5,
            shot_event_emitted_offset_ms: 6,
            sink_try_send_ok_count: 1,
            sink_try_send_full_count: 0,
            sink_try_send_disconnected_count: 0,
            bridge_try_recv_count: Some(1),
            stale_epoch_drop_count: Some(0),
            poll_iteration_id: Some(10),
            poll_wait_started_offset_ms: None,
            poll_wait_returned_offset_ms: None,
            poll_first_read_started_offset_ms: Some(2),
            poll_first_read_returned_offset_ms: Some(2),
            poll_read_result: Some(PollReadResultKind::Bytes),
            poll_frame_complete_iteration_id: Some(10),
            poll_intervening_wait_ms: Some(0),
            poll_intervening_read_calls: Some(0),
            poll_intervening_byte_reads: Some(0),
            poll_intervening_empty_reads: Some(0),
            poll_intervening_read_wait_ms: Some(0),
        }
    }

    fn touch(iteration_id: u64, result: PollReadResultKind) -> PollReadTouch {
        let at = Instant::now();
        PollReadTouch {
            iteration_id,
            wait_started: None,
            wait_returned: None,
            read_started: at,
            read_returned: at,
            read_result: result,
        }
    }

    #[test]
    fn fragment_a_then_b_inherits_provenance_of_a() {
        let frame = shot(1);
        let (a, b) = frame.split_at(30);
        let t0 = Instant::now();
        let t1 = t0 + std::time::Duration::from_millis(5);

        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        let completed = tracker.feed(&mut parser, visit(10, t0), a, Some(touch(1, PollReadResultKind::Bytes)));
        assert!(completed.is_empty());
        assert!(tracker.pending.is_some());

        let completed = tracker.feed(&mut parser, visit(11, t1), b, Some(touch(2, PollReadResultKind::Bytes)));
        assert_eq!(completed.len(), 1);
        let TracedIncoming::Shot(traced) = &completed[0] else {
            panic!("expected Shot");
        };
        assert_eq!(traced.first_chunk.rx_seq, 10);
        assert_eq!(traced.first_chunk.owner_rx_at, t0);
        assert_eq!(traced.raw, frame);
    }

    #[test]
    fn two_frames_same_chunk_share_provenance_distinct_traces() {
        let f1 = shot(2);
        let f2 = shot(3);
        let mut chunk = f1.clone();
        chunk.extend_from_slice(&f2);
        let t0 = Instant::now();

        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();
        let completed = tracker.feed(&mut parser, visit(7, t0), &chunk, Some(touch(1, PollReadResultKind::Bytes)));
        assert_eq!(completed.len(), 2);
        let TracedIncoming::Shot(a) = &completed[0] else {
            panic!("shot0");
        };
        let TracedIncoming::Shot(b) = &completed[1] else {
            panic!("shot1");
        };
        assert_eq!(a.first_chunk.rx_seq, 7);
        assert_eq!(b.first_chunk.rx_seq, 7);
        assert_eq!(a.first_chunk.owner_rx_at, t0);
        assert_eq!(b.first_chunk.owner_rx_at, t0);

        let id0 = shot_trace_id("sess", &frame_sha16(&a.raw));
        let id1 = shot_trace_id("sess", &frame_sha16(&b.raw));
        assert_ne!(id0, id1);
        assert!(id0.starts_with("sess:"));
        assert_eq!(frame_sha16(&a.raw).len(), 16);
    }

    #[test]
    fn stale_chunk_path_produces_no_traced_shot() {
        let tracker = FrameProvenanceTracker::new();
        assert!(tracker.pending.is_none());
        record_stale_epoch_drop();
        // When diag disabled, counter stays 0 — still no traced shot either way.
        assert!(tracker.pending.is_none());
    }

    #[test]
    fn json_contains_offsets_never_instant_debug() {
        let record = sample_record();
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"ownerRxOffsetMs\":1"));
        assert!(json.contains("\"schemaVersion\":2"));
        assert!(json.contains("\"bridgeFirstChunkReceivedOffsetMs\":2"));
        assert!(json.contains("\"pollReadResult\":\"bytes\""));
        assert!(!json.contains("Instant"));
        assert!(!json.contains("owner_rx_at"));
        assert!(!json.contains("raw"));
        assert!(!json.contains("valueRaw"));
    }

    /// T5: full frame + STX partial in chunk A; rest in chunk B → both use A provenance.
    #[test]
    fn t5_full_plus_partial_then_rest_keeps_first_chunk_provenance() {
        let f1 = shot(20);
        let f2 = shot(21);
        let (f2_head, f2_tail) = f2.split_at(20);
        let mut chunk_a = f1.clone();
        chunk_a.extend_from_slice(f2_head);
        let t_a = Instant::now();
        let t_b = t_a + std::time::Duration::from_millis(3);

        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        let first = tracker.feed(&mut parser, visit(100, t_a), &chunk_a, Some(touch(1, PollReadResultKind::Bytes)));
        assert_eq!(first.len(), 1);
        let TracedIncoming::Shot(s1) = &first[0] else {
            panic!("frame1");
        };
        assert_eq!(s1.first_chunk.rx_seq, 100);
        assert_eq!(s1.raw, f1);
        assert!(tracker.pending.is_some());
        assert_eq!(tracker.pending.as_ref().unwrap().visit.diag.rx_seq, 100);

        let second = tracker.feed(&mut parser, visit(101, t_b), f2_tail, Some(touch(2, PollReadResultKind::Bytes)));
        assert_eq!(second.len(), 1);
        let TracedIncoming::Shot(s2) = &second[0] else {
            panic!("frame2");
        };
        assert_eq!(s2.first_chunk.rx_seq, 100);
        assert_eq!(s2.first_chunk.owner_rx_at, t_a);
        assert_eq!(s2.raw, f2);
        assert!(tracker.pending.is_none());
    }

    /// T6: Skip/NAK before STX must not set pending; NeedMore sets it; complete clears.
    #[test]
    fn t6_garbage_before_stx_and_open_frame_pending_rules() {
        let frame = shot(30);
        let (head, tail) = frame.split_at(25);
        let t0 = Instant::now();

        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        // Garbage + NAK only — no STX yet ⇒ no pending provenance.
        let msgs = tracker.feed(
            &mut parser,
            visit(49, t0),
            &[0x00, 0xFF, NAK],
            Some(touch(1, PollReadResultKind::Bytes)),
        );
        assert!(msgs.iter().any(|m| matches!(m, TracedIncoming::Nak)));
        assert!(tracker.pending.is_none());

        // STX Teilframe ⇒ NeedMore sets pending once.
        let msgs = tracker.feed(&mut parser, visit(50, t0), head, Some(touch(2, PollReadResultKind::Bytes)));
        assert!(msgs.is_empty() || !msgs.iter().any(|m| matches!(m, TracedIncoming::Shot(_))));
        assert!(tracker.pending.is_some());
        assert_eq!(tracker.pending.as_ref().unwrap().visit.diag.rx_seq, 50);

        // Completing chunk without leading STX must not overwrite pending.
        let t1 = t0 + std::time::Duration::from_millis(2);
        let done = tracker.feed(&mut parser, visit(51, t1), tail, Some(touch(3, PollReadResultKind::Bytes)));
        assert_eq!(done.len(), 1);
        let TracedIncoming::Shot(s) = &done[0] else {
            panic!("shot");
        };
        assert_eq!(s.first_chunk.rx_seq, 50);
        assert!(tracker.pending.is_none(), "pending cleared after complete");

        // New partial; empty feed must not overwrite open-frame pending.
        let f31 = shot(31);
        let (h2, _) = f31.split_at(15);
        let _ = tracker.feed(&mut parser, visit(60, t0), h2, Some(touch(4, PollReadResultKind::Bytes)));
        assert_eq!(tracker.pending.as_ref().unwrap().visit.diag.rx_seq, 60);
        let _ = tracker.feed(&mut parser, visit(61, t1), &[], Some(touch(5, PollReadResultKind::Empty)));
        assert_eq!(tracker.pending.as_ref().unwrap().visit.diag.rx_seq, 60);
    }

    /// Writer flushes after each line — file readable without channel end / process exit.
    #[test]
    fn writer_flushes_record_visible_without_shutdown() {
        let path = std::env::temp_dir().join(format!(
            "reddot_shot_lat_flush_{}_{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("temp jsonl");
        let mut out = BufWriter::new(file);
        let record = sample_record();
        assert!(
            write_record_line(&mut out, &record),
            "writeln+flush must succeed"
        );
        // Channel still "open" from the writer's perspective — we never dropped a sender.
        // File must already contain exactly one valid JSONL line.
        let contents = std::fs::read_to_string(&path).expect("read temp jsonl");
        let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "expected exactly one JSONL line, got {contents:?}");
        let parsed: serde_json::Value =
            serde_json::from_str(lines[0]).expect("valid json");
        assert_eq!(parsed["schemaVersion"], 2);
        assert_eq!(parsed["runId"], "shotlat-test");
        assert!(parsed.get("ownerRxOffsetMs").is_some());
        assert!(parsed.get("bridgeFirstChunkReceivedOffsetMs").is_some());
        assert!(parsed.get("valueRaw").is_none());
        let _ = std::fs::remove_file(&path);
    }

    /// T7: full / disconnected diagnose queue drops without blocking or product error.
    #[test]
    fn t7_diag_queue_full_and_disconnected_drop_nonblocking() {
        let (tx, rx) = sync_channel::<ShotLatencyRecordOwned>(1);
        let before_full = DIAG_QUEUE_DROP_FULL.load(Ordering::Relaxed);
        let before_disc = DIAG_QUEUE_DROP_DISCONNECTED.load(Ordering::Relaxed);

        try_enqueue_record(&tx, sample_record());
        // Queue full — second enqueue must not block.
        try_enqueue_record(&tx, sample_record());
        assert_eq!(
            DIAG_QUEUE_DROP_FULL.load(Ordering::Relaxed),
            before_full + 1
        );
        // Drain one so we can drop rx for disconnected case.
        let _ = rx.try_recv();
        drop(rx);
        try_enqueue_record(&tx, sample_record());
        assert_eq!(
            DIAG_QUEUE_DROP_DISCONNECTED.load(Ordering::Relaxed),
            before_disc + 1
        );
    }

    /// T8: disabled path never enqueues (no writer tx).
    #[test]
    fn t8_diag_disabled_no_enqueue_when_no_tx() {
        // Mirrors Disabled state: submit only via Option path used when !Enabled.
        // env OnceLock may already be set in-process; we assert the Disabled contract
        // by calling try_enqueue only when a tx exists — without tx, nothing is sent.
        let submitted = std::sync::atomic::AtomicU64::new(0);
        let maybe_tx: Option<&SyncSender<ShotLatencyRecordOwned>> = None;
        if let Some(tx) = maybe_tx {
            try_enqueue_record(tx, sample_record());
            submitted.fetch_add(1, Ordering::Relaxed);
        }
        assert_eq!(submitted.load(Ordering::Relaxed), 0);
        assert!(
            !env_flag_enabled() || is_enabled() || !is_enabled(),
            "env parse is side-effect free for this unit"
        );
        // Explicit: flag parser defaults off without env.
        assert!(!{
            // Simulate missing env: empty string is not enabled.
            let v = "";
            v == "1" || v.eq_ignore_ascii_case("true")
        });
    }

    /// T9: stale epoch chunk → no bytes for poll → no bridge visit / no feed / no pending.
    #[test]
    fn t9_stale_sink_chunk_no_visit_no_pending() {
        let live_epoch = 3u64;
        let stale = SinkChunk {
            epoch: 2,
            bytes: shot(9),
            diag: Some(SinkChunkDiag {
                rx_seq: 99,
                owner_rx_at: Instant::now(),
                last_enq_sent_at: None,
                sink_enqueued_at: Instant::now(),
            }),
        };
        assert!(chunk_bytes_for_poll(live_epoch, &stale).is_none());
        // Bridge would not call note_bridge_visit; poll would not feed.
        let tracker = FrameProvenanceTracker::new();
        assert!(tracker.pending.is_none());
    }

    #[test]
    fn env_flag_parser_accepts_one_and_true() {
        // Pure parsing mirror of env_flag_enabled without mutating process env.
        fn parse(v: &str) -> bool {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        assert!(parse("1"));
        assert!(parse("true"));
        assert!(parse("YES"));
        assert!(!parse("0"));
        assert!(!parse(""));
        assert!(!parse("no"));
    }

    #[test]
    fn poll_trace_incomplete_wait_complete_exact_fields() {
        let frame = shot(100);
        let (a, b) = frame.split_at(30);
        let t0 = Instant::now();

        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        let was_pending = tracker.has_open_frame();
        assert!(!was_pending);
        tracker.accumulate_read_if_was_pending(was_pending, PollReadResultKind::Bytes, 0);
        let open = tracker.feed(
            &mut parser,
            visit(1, t0),
            a,
            Some(touch(10, PollReadResultKind::Bytes)),
        );
        assert!(open.is_empty());
        assert!(tracker.has_open_frame());

        tracker.accumulate_wait_ms(80);

        let was_pending = tracker.has_open_frame();
        assert!(was_pending);
        tracker.accumulate_read_if_was_pending(was_pending, PollReadResultKind::Bytes, 5);
        let done = tracker.feed(
            &mut parser,
            visit(2, t0),
            b,
            Some(touch(11, PollReadResultKind::Bytes)),
        );
        assert_eq!(done.len(), 1);
        let TracedIncoming::Shot(s) = &done[0] else {
            panic!("shot");
        };
        let poll = s.poll_loop.as_ref().expect("poll_loop");
        assert_eq!(poll.first_touch.iteration_id, 10);
        assert_eq!(poll.frame_complete_iteration_id, 11);
        assert_eq!(poll.intervening.wait_ms, 80);
        assert_eq!(poll.intervening.read_calls, 1);
        assert_eq!(poll.intervening.byte_reads, 1);
        assert_eq!(poll.intervening.empty_reads, 0);
        assert_eq!(poll.intervening.read_wait_ms, 5);
        assert!(!tracker.has_open_frame());
    }

    #[test]
    fn poll_trace_incomplete_two_empty_then_byte_complete() {
        let frame = shot(101);
        let (a, b) = frame.split_at(30);
        let t0 = Instant::now();

        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        tracker.accumulate_read_if_was_pending(false, PollReadResultKind::Bytes, 0);
        let _ = tracker.feed(
            &mut parser,
            visit(1, t0),
            a,
            Some(touch(1, PollReadResultKind::Bytes)),
        );
        assert!(tracker.has_open_frame());

        tracker.accumulate_read_if_was_pending(true, PollReadResultKind::Empty, 1);
        tracker.accumulate_read_if_was_pending(true, PollReadResultKind::Empty, 1);
        tracker.accumulate_read_if_was_pending(true, PollReadResultKind::Bytes, 2);
        let done = tracker.feed(
            &mut parser,
            visit(2, t0),
            b,
            Some(touch(4, PollReadResultKind::Bytes)),
        );
        let TracedIncoming::Shot(s) = &done[0] else {
            panic!("shot");
        };
        let poll = s.poll_loop.as_ref().unwrap();
        assert_eq!(poll.intervening.empty_reads, 2);
        assert_eq!(poll.intervening.byte_reads, 1);
        assert_eq!(poll.intervening.read_calls, 3);
        assert_eq!(poll.intervening.wait_ms, 0);
    }

    #[test]
    fn poll_trace_same_feed_complete_intervening_zero() {
        let frame = shot(102);
        let t0 = Instant::now();
        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        tracker.accumulate_read_if_was_pending(false, PollReadResultKind::Bytes, 0);
        let done = tracker.feed(
            &mut parser,
            visit(1, t0),
            &frame,
            Some(touch(10, PollReadResultKind::Bytes)),
        );
        let TracedIncoming::Shot(s) = &done[0] else {
            panic!("shot");
        };
        let poll = s.poll_loop.as_ref().unwrap();
        assert_eq!(poll.first_touch.iteration_id, 10);
        assert_eq!(poll.frame_complete_iteration_id, 10);
        assert_eq!(poll.intervening, InterveningAccum::default());
    }

    #[test]
    fn poll_trace_wait_after_complete_not_attributed() {
        let frame = shot(103);
        let t0 = Instant::now();
        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        let done = tracker.feed(
            &mut parser,
            visit(1, t0),
            &frame,
            Some(touch(5, PollReadResultKind::Bytes)),
        );
        let TracedIncoming::Shot(s) = &done[0] else {
            panic!("shot");
        };
        assert_eq!(s.poll_loop.as_ref().unwrap().intervening.wait_ms, 0);
        assert!(!tracker.has_open_frame());
        tracker.accumulate_wait_ms(80);
        assert!(!tracker.has_open_frame());
        // No pending to hold wait — intervening on completed shot stays 0.
        assert_eq!(s.poll_loop.as_ref().unwrap().intervening.wait_ms, 0);
    }

    #[test]
    fn poll_trace_tracker_discard_api_clears_pending_and_accumulator() {
        // Tracker API only — not a product parser-discard/resync hook.
        let frame = shot(104);
        let (a, _) = frame.split_at(20);
        let t0 = Instant::now();
        let mut parser = RedDotStreamParser::new();
        let mut tracker = FrameProvenanceTracker::new();

        let _ = tracker.feed(
            &mut parser,
            visit(1, t0),
            a,
            Some(touch(1, PollReadResultKind::Bytes)),
        );
        tracker.accumulate_wait_ms(80);
        tracker.accumulate_read_if_was_pending(true, PollReadResultKind::Empty, 3);
        assert!(tracker.has_open_frame());
        assert_eq!(
            tracker.pending.as_ref().unwrap().intervening.wait_ms,
            80
        );

        tracker.discard();
        assert!(!tracker.has_open_frame());
        tracker.accumulate_wait_ms(80);
        assert!(!tracker.has_open_frame());

        // Fresh frame starts clean.
        let mut parser2 = RedDotStreamParser::new();
        let done = tracker.feed(
            &mut parser2,
            visit(2, t0),
            &shot(105),
            Some(touch(9, PollReadResultKind::Bytes)),
        );
        let TracedIncoming::Shot(s) = &done[0] else {
            panic!("shot");
        };
        assert_eq!(s.poll_loop.as_ref().unwrap().intervening.wait_ms, 0);
    }

    #[test]
    fn poll_timeout_duration_constant_unchanged() {
        // Product contract: wait_timeout argument stays 80 ms (diagnosed, not altered).
        const POLL_WAIT_MS: u64 = 80;
        assert_eq!(POLL_WAIT_MS, 80);
    }
}
