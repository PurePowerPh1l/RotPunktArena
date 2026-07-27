//! C2 — Charakterisierung gegen Produktions-Fanout (`sink::apply_fanout_bytes`).
//!
//! C1-Testnamen bleiben. Soll-Fälle laufen über den echten Decision-Pfad
//! (Epoch + Pause-ACK). `LegacyFanout` dokumentiert weiterhin den Pre-C2-Istbruch
//! (kein ACK) als Kontrastmodell — nicht der Produktionspfad.

use super::sink::{
    apply_fanout_bytes, chunk_bytes_for_poll, FanoutApply, SinkChunk, SinkFanout,
};
use crate::protocol::{
    build_synthetic_shot_frame, encode_ack, Incoming, RedDotStreamParser, ACK,
};

const SHOT_LEN: usize = 59;

fn full_shot(seed: u32) -> Vec<u8> {
    let x = format!("{seed:05}");
    build_synthetic_shot_frame("10.5", "012.30", &x, "00040").unwrap()
}

/// Owner-Übergänge wie Produktions-Register/Unregister (ohne Socket).
struct FanoutOwner {
    fanout: SinkFanout,
    pause_parser: RedDotStreamParser,
    queue: Vec<SinkChunk>,
    acks: u32,
    registered_epoch: Option<u64>,
}

impl FanoutOwner {
    fn new() -> Self {
        Self {
            fanout: SinkFanout::default(),
            pause_parser: RedDotStreamParser::new(),
            queue: Vec::new(),
            acks: 0,
            registered_epoch: None,
        }
    }

    fn unregister(&mut self) {
        let prev = self.fanout.epoch;
        self.fanout = SinkFanout {
            enabled: false,
            epoch: prev.wrapping_add(1),
        };
        self.pause_parser = RedDotStreamParser::new();
        self.queue.clear();
        self.registered_epoch = None;
    }

    fn register(&mut self) {
        self.registered_epoch = Some(self.fanout.epoch);
        self.fanout = SinkFanout {
            enabled: true,
            epoch: self.fanout.epoch,
        };
    }

    fn pump_bytes(&mut self, bytes: &[u8]) {
        let fanout = self.fanout;
        match apply_fanout_bytes(fanout, &mut self.pause_parser, bytes) {
            FanoutApply::Enqueue(chunk) => self.queue.push(chunk),
            FanoutApply::PauseAck { complete_shots } => self.acks += complete_shots,
            FanoutApply::Idle => {}
        }
    }

    fn poll_consume(
        parser: &mut RedDotStreamParser,
        registered: u64,
        chunk: &SinkChunk,
    ) -> Vec<Incoming> {
        match chunk_bytes_for_poll(registered, chunk) {
            Some(bytes) => parser.push(bytes),
            None => Vec::new(),
        }
    }
}

/// Pre-C2 Kontrast: Pause ohne ACK (nicht Produktionscode).
struct LegacyFanout {
    sink_on: bool,
    pause_parser: RedDotStreamParser,
    queue: Vec<Vec<u8>>,
    acks: Vec<u8>,
}

impl LegacyFanout {
    fn new() -> Self {
        Self {
            sink_on: false,
            pause_parser: RedDotStreamParser::new(),
            queue: Vec::new(),
            acks: Vec::new(),
        }
    }

    fn unregister(&mut self) {
        self.sink_on = false;
    }

    fn register(&mut self) {
        self.sink_on = true;
    }

    fn pump_bytes(&mut self, bytes: &[u8]) {
        if self.sink_on {
            self.queue.push(bytes.to_vec());
        } else {
            let _ = self.pause_parser.push(bytes);
        }
    }
}

#[test]
fn t11_after_unregister_full_frame_acks_and_discards_no_sink_chunk() {
    let mut o = FanoutOwner::new();
    o.register();
    o.unregister();
    assert!(!o.fanout.enabled);

    o.pump_bytes(&full_shot(11));
    assert_eq!(o.acks, 1, "T11: Pause-ACK über apply_fanout_bytes");
    assert!(o.queue.is_empty(), "T11: kein SinkChunk nach Unregister");
}

#[test]
fn t2_unregister_then_full_shot_ack_discard_zero_queue() {
    let mut o = FanoutOwner::new();
    o.register();
    o.unregister();
    o.pump_bytes(&full_shot(2));
    assert_eq!(o.acks, 1);
    assert!(o.queue.is_empty());
    assert_eq!(encode_ack(), vec![ACK]);
}

/// Kontrast Pre-C2: Legacy ohne ACK — Produktionspfad ist T2/T11 oben.
#[test]
fn t2_today_legacy_pause_frame_no_ack_p0_breach() {
    let mut leg = LegacyFanout::new();
    leg.register();
    leg.unregister();
    leg.pump_bytes(&full_shot(22));
    assert!(
        leg.acks.is_empty(),
        "Pre-C2-Modell: Pause ohne ACK (Kontrast; Prod = apply_fanout_bytes)"
    );
    assert!(leg.queue.is_empty());
}

#[test]
fn t12_after_register_b_chunks_carry_registered_epoch_b() {
    let mut o = FanoutOwner::new();
    o.register();
    o.unregister();
    o.register();
    let reg = o.registered_epoch.expect("registered");
    assert_eq!(reg, 1);

    o.pump_bytes(&[0x02, 0x20]);
    o.pump_bytes(&full_shot(12)[..10]);
    assert!(!o.queue.is_empty());
    for chunk in &o.queue {
        assert_eq!(chunk.epoch, reg, "T12: Chunk.epoch == registered_epoch(B)");
    }
}

#[test]
fn t4_stale_epoch_chunk_dropped_without_resetting_poll_parser() {
    let mut o = FanoutOwner::new();
    o.register();
    let epoch_a = o.fanout.epoch;
    o.pump_bytes(&full_shot(40)[..30]);
    let stale = SinkChunk {
        epoch: epoch_a,
        bytes: full_shot(40)[..30].to_vec(),
        diag: None,
    };
    o.unregister();
    o.register();
    let reg_b = o.registered_epoch.unwrap();
    assert_ne!(stale.epoch, reg_b);

    let mut poll_parser = RedDotStreamParser::new();
    let live_prefix = full_shot(41)[..20].to_vec();
    let _ = poll_parser.push(&live_prefix);

    let from_stale = FanoutOwner::poll_consume(&mut poll_parser, reg_b, &stale);
    assert!(from_stale.is_empty(), "stale darf nicht in Poll-Parser");

    let rest = full_shot(41)[20..].to_vec();
    let live_chunk = SinkChunk {
        epoch: reg_b,
        bytes: rest,
        diag: None,
    };
    let msgs = FanoutOwner::poll_consume(&mut poll_parser, reg_b, &live_chunk);
    assert!(
        msgs.iter().any(|m| matches!(m, Incoming::ShotFrame(_))),
        "T4: Poll-Parser bleibt für aktuelle Epoch intakt"
    );
}

#[test]
fn t10_split_30_then_29_across_unregister_must_not_form_shot_in_b() {
    let frame = full_shot(10);
    assert_eq!(frame.len(), SHOT_LEN);
    let (head, tail) = frame.split_at(30);

    let mut o = FanoutOwner::new();
    o.register();
    o.pump_bytes(head);
    o.unregister();
    o.register();
    let reg = o.registered_epoch.unwrap();

    let mut poll = RedDotStreamParser::new();
    let stale_residual = SinkChunk {
        epoch: reg.wrapping_sub(1),
        bytes: head.to_vec(),
        diag: None,
    };
    assert!(FanoutOwner::poll_consume(&mut poll, reg, &stale_residual).is_empty());

    o.pump_bytes(tail);
    assert_eq!(o.queue.len(), 1);
    assert_eq!(o.queue[0].epoch, reg);
    let msgs = FanoutOwner::poll_consume(&mut poll, reg, &o.queue[0]);
    assert!(
        !msgs.iter().any(|m| matches!(m, Incoming::ShotFrame(_))),
        "T10: 29 Restbytes allein → kein Shot in B"
    );

    let b_shot = full_shot(99);
    let chunk = SinkChunk {
        epoch: reg,
        bytes: b_shot,
        diag: None,
    };
    let msgs = FanoutOwner::poll_consume(&mut poll, reg, &chunk);
    let shots: Vec<_> = msgs
        .into_iter()
        .filter(|m| matches!(m, Incoming::ShotFrame(_)))
        .collect();
    assert_eq!(shots.len(), 1);
}

#[test]
fn t10_danger_single_parser_concat_30_plus_29_forms_shot() {
    let frame = full_shot(10);
    let (head, tail) = frame.split_at(30);
    let mut p = RedDotStreamParser::new();
    let _ = p.push(head);
    let msgs = p.push(tail);
    assert!(
        msgs.iter().any(|m| matches!(m, Incoming::ShotFrame(_))),
        "ohne Epoch-Grenze kann ein Parser Teilframes zusammensetzen"
    );
}

#[test]
fn t8_frame_before_register_acks_discard_never_queued_for_later() {
    let mut o = FanoutOwner::new();
    assert!(!o.fanout.enabled);
    o.pump_bytes(&full_shot(8));
    assert_eq!(o.acks, 1);
    assert!(o.queue.is_empty(), "T8: nie still für spätere Serie queue");
    o.register();
    assert!(o.queue.is_empty());
}

#[test]
fn t6_chunk_enqueued_before_register_b_must_not_reach_poll_as_b1() {
    let mut o = FanoutOwner::new();
    o.register();
    let epoch_a = o.fanout.epoch;
    let residual = SinkChunk {
        epoch: epoch_a,
        bytes: full_shot(6),
        diag: None,
    };
    o.unregister();
    o.register();
    let reg_b = o.registered_epoch.unwrap();

    let mut poll = RedDotStreamParser::new();
    let msgs = FanoutOwner::poll_consume(&mut poll, reg_b, &residual);
    assert!(msgs.is_empty(), "T6: Vor-Register-Chunk nicht als B-Shot");
}

#[test]
fn t9_after_unregister_full_frame_never_enters_queue_for_b() {
    let mut o = FanoutOwner::new();
    o.register();
    o.unregister();
    o.pump_bytes(&full_shot(9));
    assert!(o.queue.is_empty());
    assert_eq!(o.acks, 1);
    o.register();
    assert!(o.queue.is_empty(), "T9: Endgrenzen-Frame nicht für B gequeued");
}

#[test]
fn fanout_unregister_is_single_transition_then_pump_sees_disabled() {
    let mut o = FanoutOwner::new();
    o.register();
    let e0 = o.fanout.epoch;
    o.unregister();
    assert_eq!(
        o.fanout,
        SinkFanout {
            enabled: false,
            epoch: e0 + 1
        }
    );
    o.pump_bytes(&full_shot(1));
    assert!(o.queue.is_empty());
}

/// T3 Transportanteil: Pause-Vollframe nach Unregister → ACK/discard; Register B → Queue leer.
#[test]
fn t3_pause_shot_after_unregister_never_queued_for_series_b() {
    let mut o = FanoutOwner::new();
    o.register(); // Serie A
    o.unregister();
    o.pump_bytes(&full_shot(30));
    assert_eq!(o.acks, 1);
    assert!(o.queue.is_empty());
    o.register(); // Serie B
    assert!(
        o.queue.is_empty(),
        "T3: Pause-Schuss nicht als Channel-Rest für B"
    );
    let reg = o.registered_epoch.unwrap();
    o.pump_bytes(&full_shot(31)[..5]);
    assert!(o.queue.iter().all(|c| c.epoch == reg));
}
