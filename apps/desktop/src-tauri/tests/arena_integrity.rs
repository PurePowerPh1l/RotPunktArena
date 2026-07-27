//! Arena Core integrity tests — commit-before-UI, dedupe, replay pipeline.

use reddot_desktop_lib::{
    build_synthetic_shot_frame, encode_enq, parse_hex_capture, ArenaDb, Incoming, IngestOutcome,
    RedDotStreamParser, ReplayTransport, Transport, PARSER_VERSION,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_frame(i: u32) -> Vec<u8> {
    let x = format!("{i:05}");
    assert_eq!(x.len(), 5);
    build_synthetic_shot_frame("10.5", "012.30", &x, "00040").unwrap()
}

fn temp_db_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("reddot-arena-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("t.sqlite")
}

#[test]
fn migrations_and_parser_version() {
    let _db = ArenaDb::open_in_memory().unwrap();
    assert_eq!(PARSER_VERSION, "reddot-stx-v1");
}

#[test]
fn duplicate_frames_yield_exact_shot_count() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("Tester", None, None, None).unwrap();

    const UNIQUE: usize = 500;
    const DUP_EACH: usize = 1; // 1000 frames → 500 accepted shots

    let mut accepted = 0usize;
    let mut duplicates = 0usize;
    let mut ui_emits = 0usize;

    for i in 0..UNIQUE {
        let frame = unique_frame(i as u32);
        for _ in 0..=DUP_EACH {
            match db
                .ingest_raw_frame(&session.id, &frame, "test", None)
                .unwrap()
            {
                IngestOutcome::Accepted(_) => {
                    accepted += 1;
                    ui_emits += 1;
                }
                IngestOutcome::Duplicate { .. } => duplicates += 1,
                IngestOutcome::ParseFailed { error, .. } => panic!("parse failed: {error}"),
                IngestOutcome::LimitReached { .. } => panic!("unexpected limit"),
                IngestOutcome::SessionInactive { .. } => panic!("unexpected SessionInactive"),
            }
        }
    }

    assert_eq!(accepted, UNIQUE);
    assert_eq!(duplicates, UNIQUE * DUP_EACH);
    assert_eq!(ui_emits, UNIQUE);
    assert_eq!(db.count_session_shots(&session.id).unwrap() as usize, UNIQUE);
    assert_eq!(db.count_frames().unwrap() as usize, UNIQUE);
    assert_eq!(db.count_events_kind("shot_received").unwrap() as usize, UNIQUE);
}

#[test]
fn commit_before_ui_fail_closed_on_bad_session() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let frame = unique_frame(1);
    match db
        .ingest_raw_frame("missing-session", &frame, "test", None)
        .unwrap()
    {
        IngestOutcome::SessionInactive { session_id } => {
            assert_eq!(session_id, "missing-session");
        }
        o => panic!("expected SessionInactive, got {o:?}"),
    }
    assert_eq!(db.count_all_shots().unwrap(), 0);
    assert_eq!(db.count_frames().unwrap(), 0);
}

#[test]
fn reopen_shows_shot_exactly_once() {
    let path = temp_db_path();
    let dir = path.parent().unwrap().to_path_buf();

    let session_id = {
        let mut db = ArenaDb::open(&path).unwrap();
        let session = db.start_session("Recover", None, None, None).unwrap();
        let frame = unique_frame(42);
        match db.ingest_raw_frame(&session.id, &frame, "device", None).unwrap() {
            IngestOutcome::Accepted(_) => {}
            other => panic!("expected accepted, got {other:?}"),
        }
        let unclean = db.list_unclean_sessions().unwrap();
        assert!(unclean.contains(&session.id));
        session.id
    };

    {
        let mut db = ArenaDb::open(&path).unwrap();
        assert_eq!(db.count_session_shots(&session_id).unwrap(), 1);
        let frame = unique_frame(42);
        match db.ingest_raw_frame(&session_id, &frame, "device", None).unwrap() {
            IngestOutcome::Duplicate { .. } => {}
            other => panic!("expected duplicate after restart, got {other:?}"),
        }
        assert_eq!(db.count_session_shots(&session_id).unwrap(), 1);
        assert!(db.list_unclean_sessions().unwrap().contains(&session_id));
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn golden_hex_replay_uses_same_ingest_pipeline() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/captures/synthetic-shot.hex");
    let text = std::fs::read_to_string(&fixture).expect("golden fixture");
    let chunks = parse_hex_capture(&text).unwrap();
    assert!(chunks.iter().any(|c| c.first() == Some(&0x02)));

    let mut transport = ReplayTransport::from_hex_str(&text, "synthetic-shot").unwrap();
    transport.open().unwrap();

    let mut parser = RedDotStreamParser::new();
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("Replay", None, None, None).unwrap();

    let mut accepted = 0;
    for _ in 0..16 {
        transport.write_all(&encode_enq()).unwrap();
        let mut buf = [0u8; 128];
        let n = transport
            .read_timeout(&mut buf, std::time::Duration::from_millis(5))
            .unwrap();
        if n == 0 {
            continue;
        }
        for msg in parser.push(&buf[..n]) {
            if let Incoming::ShotFrame(raw) = msg {
                match db.ingest_raw_frame(&session.id, &raw, "replay", None).unwrap() {
                    IngestOutcome::Accepted(_) => accepted += 1,
                    IngestOutcome::Duplicate { .. } => {}
                    IngestOutcome::ParseFailed { error, .. } => panic!("{error}"),
                    IngestOutcome::LimitReached { .. } => panic!("unexpected limit"),
                    IngestOutcome::SessionInactive { .. } => panic!("unexpected SessionInactive"),
                }
            }
        }
    }

    let mut transport2 = ReplayTransport::from_hex_str(&text, "synthetic-shot-2").unwrap();
    transport2.open().unwrap();
    let mut parser2 = RedDotStreamParser::new();
    for _ in 0..16 {
        transport2.write_all(&encode_enq()).unwrap();
        let mut buf = [0u8; 128];
        let n = transport2
            .read_timeout(&mut buf, std::time::Duration::from_millis(5))
            .unwrap();
        if n == 0 {
            continue;
        }
        for msg in parser2.push(&buf[..n]) {
            if let Incoming::ShotFrame(raw) = msg {
                let _ = db.ingest_raw_frame(&session.id, &raw, "replay", None).unwrap();
            }
        }
    }

    assert_eq!(accepted, 1);
    assert_eq!(db.count_session_shots(&session.id).unwrap(), 1);
}

#[test]
fn device_sequence_dedupes_even_if_bytes_differ() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("DevSeq", None, None, None).unwrap();
    let a = unique_frame(1);
    let b = unique_frame(2);
    assert_ne!(a, b);

    match db
        .ingest_raw_frame(&session.id, &a, "device", Some(7))
        .unwrap()
    {
        IngestOutcome::Accepted(_) => {}
        o => panic!("{o:?}"),
    }
    match db
        .ingest_raw_frame(&session.id, &b, "device", Some(7))
        .unwrap()
    {
        IngestOutcome::Duplicate { .. } => {}
        o => panic!("expected duplicate by device_sequence, got {o:?}"),
    }
    assert_eq!(db.count_session_shots(&session.id).unwrap(), 1);
}

#[test]
fn competition_max_shots_rejects_extra() {
    use reddot_desktop_lib::CreateCompetition;

    let mut db = ArenaDb::open_in_memory().unwrap();
    let comp = db
        .create_competition(CreateCompetition {
            name: "LG 5".into(),
            date: "2026-07-22".into(),
            discipline: "Luftgewehr".into(),
            max_shots: 5,
            scoring_mode: "ringe".into(),
            nachkauf_enabled: false,
            nachkauf_shots: 0,
            team_scoring_enabled: false,
            team_count: 3,
            kind: "competition".into(),
        })
        .unwrap();
    let session = db
        .start_session("Wettkämpfer", Some(&comp.id), None, None)
        .unwrap();

    for i in 0..5 {
        let frame = unique_frame(i as u32);
        match db
            .ingest_raw_frame(&session.id, &frame, "test", None)
            .unwrap()
        {
            IngestOutcome::Accepted(a) => assert_eq!(a.shot_index, i + 1),
            o => panic!("expected accepted #{}, got {o:?}", i + 1),
        }
    }
    assert_eq!(db.count_session_shots(&session.id).unwrap(), 5);

    let extra = unique_frame(99);
    match db
        .ingest_raw_frame(&session.id, &extra, "test", None)
        .unwrap()
    {
        IngestOutcome::LimitReached {
            max_shots,
            current_shots,
        } => {
            assert_eq!(max_shots, 5);
            assert_eq!(current_shots, 5);
        }
        o => panic!("expected LimitReached, got {o:?}"),
    }
    assert_eq!(db.count_session_shots(&session.id).unwrap(), 5);
}

#[test]
fn recovery_autosave_list_close_and_rehydrate_shots() {
    let path = temp_db_path();
    let dir = path.parent().unwrap().to_path_buf();

    let session_id = {
        let mut db = ArenaDb::open(&path).unwrap();
        let session = db.start_session("RecoveryGate", None, None, None).unwrap();
        // Session start already writes autosave marker.
        let list0 = db.list_recovery_sessions().unwrap();
        assert_eq!(list0.len(), 1);
        assert!(list0[0].last_autosave_at.is_some());
        assert!(list0[0].last_autosave_sequence.is_some());

        for i in 0..3u32 {
            let frame = unique_frame(i);
            match db
                .ingest_raw_frame(&session.id, &frame, "test", None)
                .unwrap()
            {
                IngestOutcome::Accepted(_) => {}
                o => panic!("expected accepted, got {o:?}"),
            }
        }
        let list = db.list_recovery_sessions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, session.id);
        assert_eq!(list[0].shooter_name, "RecoveryGate");
        assert_eq!(list[0].shot_count, 3);
        assert!(list[0].competition_id.is_none());
        assert_eq!(list[0].recovery_state, "interrupted");
        // Autosave sequence advanced with accepted shots (events).
        let seq = list[0].last_autosave_sequence.unwrap();
        assert!(seq >= 3, "expected autosave sequence >= 3, got {seq}");

        let shots = db.load_session_ui_shots(&session.id).unwrap();
        assert_eq!(shots.len(), 3);
        assert_eq!(shots[0].shot_index, 1);
        assert_eq!(shots[2].shot_index, 3);
        assert!(shots[0].series_total > 0.0);
        assert_eq!(
            shots[2].series_total,
            shots.iter().map(|s| s.value_display).sum::<f64>()
        );

        // Simulate crash: drop without end_session — open + autosave remains.
        assert!(db.list_unclean_sessions().unwrap().contains(&session.id));
        session.id
    };

    {
        let mut db = ArenaDb::open(&path).unwrap();
        assert_eq!(db.list_recovery_sessions().unwrap().len(), 1);
        let shots = db.load_session_ui_shots(&session_id).unwrap();
        assert_eq!(shots.len(), 3);

        db.close_interrupted_session(&session_id).unwrap();
        assert!(db.list_unclean_sessions().unwrap().is_empty());
        assert!(db.list_recovery_sessions().unwrap().is_empty());

        let session = db.get_session(&session_id).unwrap().unwrap();
        assert!(session.ended_at.is_some());
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn emergency_export_zip_contains_db_and_manifest() {
    use std::io::Read;
    use zip::ZipArchive;

    let path = temp_db_path();
    let dir = path.parent().unwrap().to_path_buf();
    let zip_path = dir.join("emergency.zip");

    {
        let mut db = ArenaDb::open(&path).unwrap();
        let session = db.start_session("Export", None, None, None).unwrap();
        let frame = unique_frame(7);
        match db
            .ingest_raw_frame(&session.id, &frame, "test", None)
            .unwrap()
        {
            IngestOutcome::Accepted(_) => {}
            o => panic!("{o:?}"),
        }

        let staging = dir.join("copy.sqlite");
        db.vacuum_into(&staging).unwrap();
        assert!(staging.is_file());

        let unclean = db.list_unclean_sessions().unwrap();
        let events = db.dump_events_jsonl(&unclean).unwrap();
        assert!(events.contains("shot_received") || events.contains("session_started"));

        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts =
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("reddot.sqlite", opts).unwrap();
        let mut db_bytes = std::fs::File::open(&staging).unwrap();
        std::io::copy(&mut db_bytes, &mut zip).unwrap();
        zip.start_file("manifest.json", opts).unwrap();
        let manifest = serde_json::json!({
            "schemaVersion": db.schema_version().unwrap(),
            "uncleanSessionIds": unclean,
        });
        use std::io::Write;
        zip.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
            .unwrap();
        zip.start_file("events.jsonl", opts).unwrap();
        zip.write_all(events.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let file = std::fs::File::open(&zip_path).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "reddot.sqlite"));
    assert!(names.iter().any(|n| n == "manifest.json"));
    assert!(names.iter().any(|n| n == "events.jsonl"));

    let mut manifest_txt = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest_txt)
        .unwrap();
    assert!(manifest_txt.contains("uncleanSessionIds"));
    assert!(manifest_txt.contains("schemaVersion"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn competition_nachkauf_full_series_best_of() {
    use reddot_desktop_lib::CreateCompetition;

    let mut db = ArenaDb::open_in_memory().unwrap();
    let person = db
        .create_person(reddot_desktop_lib::CreatePerson {
            first_name: "Max".into(),
            last_name: "Mustermann".into(),
            club: None,
        })
        .unwrap();
    let comp = db
        .create_competition(CreateCompetition {
            name: "LG Nachkauf".into(),
            date: "2026-07-22".into(),
            discipline: "Luftgewehr".into(),
            max_shots: 3,
            scoring_mode: "ringe".into(),
            nachkauf_enabled: true,
            nachkauf_shots: 0,
            team_scoring_enabled: false,
            team_count: 3,
            kind: "competition".into(),
        })
        .unwrap();
    let entry = db.add_entry(&comp.id, &person.id).unwrap();

    // Serie 1
    db.activate_entry(&entry.id).unwrap();
    let session1 = db
        .start_session("Wettkämpfer", Some(&comp.id), Some(&entry.id), None)
        .unwrap();
    for i in 0..3 {
        let frame = unique_frame(i as u32);
        match db
            .ingest_raw_frame(&session1.id, &frame, "test", None)
            .unwrap()
        {
            IngestOutcome::Accepted(a) => assert_eq!(a.shot_index, i + 1),
            o => panic!("expected accepted #{}, got {o:?}", i + 1),
        }
    }
    let extra = unique_frame(50);
    match db
        .ingest_raw_frame(&session1.id, &extra, "test", None)
        .unwrap()
    {
        IngestOutcome::LimitReached {
            max_shots,
            current_shots,
        } => {
            assert_eq!(max_shots, 3);
            assert_eq!(current_shots, 3);
        }
        o => panic!("expected LimitReached after serie 1, got {o:?}"),
    }
    db.end_session(&session1.id).unwrap();
    db.set_entry_status(&entry.id, "done").unwrap();

    // Nachkauf: neue Session, Limit wieder = maxShots
    let after_nk = db.activate_entry(&entry.id).unwrap();
    assert_eq!(after_nk.nachkauf_purchased, 1);
    let session2 = db
        .start_session("Wettkämpfer", Some(&comp.id), Some(&entry.id), None)
        .unwrap();
    assert_ne!(session2.id, session1.id);
    for i in 0..3 {
        let frame = unique_frame(100 + i as u32);
        match db
            .ingest_raw_frame(&session2.id, &frame, "test", None)
            .unwrap()
        {
            IngestOutcome::Accepted(_) => {}
            o => panic!("expected accepted nachkauf #{}, got {o:?}", i + 1),
        }
    }
    let extra2 = unique_frame(199);
    match db
        .ingest_raw_frame(&session2.id, &extra2, "test", None)
        .unwrap()
    {
        IngestOutcome::LimitReached { max_shots, .. } => assert_eq!(max_shots, 3),
        o => panic!("expected LimitReached after nachkauf, got {o:?}"),
    }
    db.end_session(&session2.id).unwrap();
    db.set_entry_status(&entry.id, "done").unwrap();

    let series = db.list_entry_series(&entry.id).unwrap();
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].series_index, 1);
    assert!(!series[0].is_nachkauf);
    assert_eq!(series[1].series_index, 2);
    assert!(series[1].is_nachkauf);
    assert_eq!(series.iter().filter(|s| s.is_best).count(), 1);

    let results = db.list_competition_results(&comp.id).unwrap();
    assert_eq!(results.len(), 1);
    let best = &results[0];
    assert_eq!(best.shot_count, 3);
    assert!(best.session_id.is_some());
    let detail = db.get_entry_result(&entry.id).unwrap().unwrap();
    assert_eq!(detail.series.len(), 2);
    assert_eq!(detail.max_shots, 3);
    assert_eq!(detail.summary.session_id, best.session_id);

    // Ohne Nachkauf: zweiter Start nach done schlägt fehl
    let comp2 = db
        .create_competition(CreateCompetition {
            name: "LG ohne NK".into(),
            date: "2026-07-22".into(),
            discipline: "Luftgewehr".into(),
            max_shots: 2,
            scoring_mode: "ringe".into(),
            nachkauf_enabled: false,
            nachkauf_shots: 0,
            team_scoring_enabled: false,
            team_count: 3,
            kind: "competition".into(),
        })
        .unwrap();
    let entry2 = db.add_entry(&comp2.id, &person.id).unwrap();
    db.activate_entry(&entry2.id).unwrap();
    let s = db
        .start_session("Wettkämpfer", Some(&comp2.id), Some(&entry2.id), None)
        .unwrap();
    db.end_session(&s.id).unwrap();
    db.set_entry_status(&entry2.id, "done").unwrap();
    let err = db.activate_entry(&entry2.id).unwrap_err();
    assert!(
        err.contains("bereits beendet"),
        "expected done block without nachkauf, got: {err}"
    );
}

#[test]
fn parse_failed_stores_frame_and_event_without_shot() {
    let mut db = ArenaDb::open_in_memory().unwrap();
    let session = db.start_session("Bad", None, None, None).unwrap();
    let bad = b"\x02not-a-valid-stx-frame\x03";

    match db
        .ingest_raw_frame(&session.id, bad, "test", None)
        .unwrap()
    {
        IngestOutcome::ParseFailed { frame_id, error } => {
            assert!(!frame_id.is_empty());
            assert!(!error.is_empty());
        }
        o => panic!("expected ParseFailed, got {o:?}"),
    }

    assert_eq!(db.count_session_shots(&session.id).unwrap(), 0);
    assert_eq!(db.count_frames().unwrap(), 1);
    assert_eq!(db.count_events_kind("frame_parse_error").unwrap(), 1);
}

#[test]
fn hybrid_vacuum_snapshots_on_session_and_every_n_shots() {
    use reddot_desktop_lib::{SNAPSHOT_EVERY_N_SHOTS, SNAPSHOT_SUBDIR};

    let path = temp_db_path();
    let dir = path.parent().unwrap().to_path_buf();
    let snap_dir = dir.join(SNAPSHOT_SUBDIR);

    {
        let mut db = ArenaDb::open(&path).unwrap();
        let session = db.start_session("SnapTester", None, None, None).unwrap();
        assert!(
            snap_dir.is_dir(),
            "session start should create {SNAPSHOT_SUBDIR}/"
        );
        let after_start = count_session_snap_files(&snap_dir, &session.id);
        assert!(
            after_start >= 1,
            "expected ≥1 session snapshot after start, got {after_start}"
        );
        assert!(snap_dir.join("latest.sqlite").is_file());

        let n = SNAPSHOT_EVERY_N_SHOTS as u32;
        for i in 0..n {
            let frame = unique_frame(i);
            match db
                .ingest_raw_frame(&session.id, &frame, "test", None)
                .unwrap()
            {
                IngestOutcome::Accepted(_) => {}
                o => panic!("expected Accepted at shot {}, got {o:?}", i + 1),
            }
        }
        assert_eq!(db.count_session_shots(&session.id).unwrap(), i64::from(n));

        let after_n = count_session_snap_files(&snap_dir, &session.id);
        assert!(
            after_n > after_start,
            "expected cadence snapshot after {n} shots (was {after_start}, now {after_n})"
        );

        db.end_session(&session.id).unwrap();
        let after_end = count_session_snap_files(&snap_dir, &session.id);
        assert!(
            after_end > after_n,
            "expected end-session snapshot (was {after_n}, now {after_end})"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

fn count_session_snap_files(snap_dir: &std::path::Path, session_id: &str) -> usize {
    let needle = format!("session-{session_id}-");
    std::fs::read_dir(snap_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|n| n.starts_with(&needle) && n.ends_with(".sqlite"))
                })
                .count()
        })
        .unwrap_or(0)
}
