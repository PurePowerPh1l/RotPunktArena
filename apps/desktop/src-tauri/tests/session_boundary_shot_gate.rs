//! Session-boundary shot gate — Arena-/Persistenzschicht (C1–C3).
//!
//! Fanout/Epoch/Pause-ACK: `connection::session_boundary_shot_gate_tests`.

use reddot_desktop_lib::{
    build_synthetic_shot_frame, ArenaDb, IngestOutcome,
};

fn unique_frame(i: u32) -> Vec<u8> {
    let x = format!("{i:05}");
    assert_eq!(x.len(), 5);
    build_synthetic_shot_frame("10.5", "012.30", &x, "00040").unwrap()
}

fn accept_count(db: &mut ArenaDb, session_id: &str, frame: &[u8]) -> IngestOutcome {
    db.ingest_raw_frame(session_id, frame, "test", None)
        .expect("ingest")
}

/// T1: offene Session + vollständiger Frame → genau 1 Shot / 1 shot_received (UI-Contract = Accepted).
#[test]
fn t1_open_session_full_shot_persists_exactly_once() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("T1", None, None, None).unwrap();
    let frame = unique_frame(1);

    match accept_count(&mut db, &session.id, &frame) {
        IngestOutcome::Accepted(a) => assert_eq!(a.shot_index, 1),
        o => panic!("expected Accepted, got {o:?}"),
    }
    match accept_count(&mut db, &session.id, &frame) {
        IngestOutcome::Duplicate { .. } => {}
        o => panic!("expected Duplicate on resend, got {o:?}"),
    }

    assert_eq!(db.count_session_shots(&session.id).unwrap(), 1);
    assert_eq!(db.count_events_kind("shot_received").unwrap(), 1);
}

/// T7: Resend/Duplikat in aktiver Session → kein zweiter Score.
#[test]
fn t7_duplicate_resend_in_active_session_no_second_score() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("T7", None, None, None).unwrap();
    let frame = unique_frame(7);

    assert!(matches!(
        accept_count(&mut db, &session.id, &frame),
        IngestOutcome::Accepted(_)
    ));
    assert!(matches!(
        accept_count(&mut db, &session.id, &frame),
        IngestOutcome::Duplicate { .. }
    ));
    assert!(matches!(
        accept_count(&mut db, &session.id, &frame),
        IngestOutcome::Duplicate { .. }
    ));
    assert_eq!(db.count_session_shots(&session.id).unwrap(), 1);
}

/// T5: Ingest nach `end_session` → SessionInactive, kein frames/shots/shot_received.
#[test]
fn t5_ingest_after_end_session_must_not_accept() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("T5", None, None, None).unwrap();
    let before_shots = db.count_session_shots(&session.id).unwrap();
    let before_events = db.count_events_kind("shot_received").unwrap();
    let before_frames = db.count_frames().unwrap();

    db.end_session(&session.id).unwrap();
    let ended = db.get_session(&session.id).unwrap().unwrap();
    assert!(ended.ended_at.is_some());

    let frame = unique_frame(51);
    match accept_count(&mut db, &session.id, &frame) {
        IngestOutcome::SessionInactive { session_id } => {
            assert_eq!(session_id, session.id);
        }
        o => panic!("expected SessionInactive, got {o:?}"),
    }
    assert_eq!(db.count_session_shots(&session.id).unwrap(), before_shots);
    assert_eq!(db.count_events_kind("shot_received").unwrap(), before_events);
    assert_eq!(db.count_frames().unwrap(), before_frames);
}

/// T3 Persistenz: nach Ende A → SessionInactive auf A; B startet leer und bekommt eigenen Schuss.
#[test]
fn t3_session_b_starts_empty_and_only_own_ingest_becomes_b1() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let a = db.start_session("T3-A", None, None, None).unwrap();
    let pause_frame = unique_frame(30);
    db.end_session(&a.id).unwrap();

    let b = db.start_session("T3-B", None, None, None).unwrap();
    assert_eq!(db.count_session_shots(&b.id).unwrap(), 0);

    match accept_count(&mut db, &a.id, &pause_frame) {
        IngestOutcome::SessionInactive { .. } => {}
        o => panic!("ended A must be SessionInactive, got {o:?}"),
    }
    assert_eq!(db.count_session_shots(&b.id).unwrap(), 0);
    assert_eq!(db.count_session_shots(&a.id).unwrap(), 0);

    let b_frame = unique_frame(31);
    match accept_count(&mut db, &b.id, &b_frame) {
        IngestOutcome::Accepted(acc) => assert_eq!(acc.shot_index, 1),
        o => panic!("expected B/1, got {o:?}"),
    }
    assert_eq!(db.count_session_shots(&b.id).unwrap(), 1);
}

/// T9: vor Ende in A akzeptiert; nach Ende Late-Ingest → SessionInactive; B bleibt leer davon.
#[test]
fn t9_shot_fully_ingested_before_end_belongs_to_session_a() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let a = db.start_session("T9-A", None, None, None).unwrap();
    let frame = unique_frame(90);
    match accept_count(&mut db, &a.id, &frame) {
        IngestOutcome::Accepted(acc) => assert_eq!(acc.shot_index, 1),
        o => panic!("{o:?}"),
    }
    db.end_session(&a.id).unwrap();
    assert_eq!(db.count_session_shots(&a.id).unwrap(), 1);

    let late = unique_frame(91);
    match accept_count(&mut db, &a.id, &late) {
        IngestOutcome::SessionInactive { .. } => {}
        o => panic!("post-end ingest must be SessionInactive, got {o:?}"),
    }
    assert_eq!(db.count_session_shots(&a.id).unwrap(), 1);

    let b = db.start_session("T9-B", None, None, None).unwrap();
    assert_eq!(db.count_session_shots(&b.id).unwrap(), 0);
    assert_eq!(db.count_session_shots(&a.id).unwrap(), 1);
}

/// T6: fehlende Session-id → SessionInactive, kein Persist.
#[test]
fn t6_no_session_b_id_before_start_missing_session_fail_closed() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let frame = unique_frame(60);
    match db
        .ingest_raw_frame("not-yet-started-b", &frame, "test", None)
        .unwrap()
    {
        IngestOutcome::SessionInactive { session_id } => {
            assert_eq!(session_id, "not-yet-started-b");
        }
        o => panic!("expected SessionInactive, got {o:?}"),
    }
    assert_eq!(db.count_all_shots().unwrap(), 0);
    assert_eq!(db.count_frames().unwrap(), 0);
}

// Poll session_id ↔ Engine mismatch: kein leichtes Fixture ohne StandEngine-Harness.
// Abgedeckt produktiv in handle_shot_frame via poll_session_accepting; Arena-Wahrheit = T5.
// Folgebedarf (optional C4): Unit-Test mit Engine-Snapshot, kein Architektur-Umbau jetzt.
