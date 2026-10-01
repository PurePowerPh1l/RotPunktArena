# ADR: WAL-safe DB Snapshots (`VACUUM INTO`)

Status: Accepted  
Date: 2026-07-23

## Context

RedDot Arena keeps the live database in SQLite **WAL** mode with `PRAGMA synchronous=FULL` (see [ADR 0009](./0009-shot-durability.md)). Committed work can still sit in the `-wal` file until a checkpoint. Copying only `reddot.sqlite` (or a naive filesystem copy of the live DB) can therefore produce an **incomplete / inconsistent** backup.

Autosave markers (`last_autosave_sequence` / `last_autosave_at`) track progress inside the DB but do not create a restorable file image.

## Decision

1. **Mechanism:** create file snapshots only with `VACUUM INTO` (after `PRAGMA wal_checkpoint(TRUNCATE)`), never by copying the live `.sqlite` / `.wal` pair as a backup.
2. **Prerequisite:** product connections use `FULL` in `db/mod.rs`; the shot durability contract includes OS/power failure subject to correct storage synchronization.
3. **Hybrid triggers:**
   - Session **start** and **end** (boundary)
   - Every **N = 100** accepted shots (`shot_index % 100 == 0`), outside the ingest transaction
4. **Location:** `{app_data}/snapshots/` next to the live DB; per-session files plus a rolling `latest.sqlite` (copy of the last VACUUM artifact — not a copy of the live DB).
5. **Failure policy:** snapshot errors are best-effort (log + continue). They must not fail-close ingest or block session start/end.
6. **Writer:** asynchronous snapshots currently open dedicated Rust background connections; synchronous helpers use the caller's connection. The frontend never writes snapshots (see [ADR 0002](./0002-single-writer.md)). Background lifecycle coordination is tracked as F11/F01 in the hardening review.

## Anti-patterns

- `std::fs::copy("reddot.sqlite", …)` / Explorer-Kopie der Live-DB als Backup
- Snapshot **inside** the Arena ingest transaction
- Snapshot on every single shot (too expensive)

## Consequences

- Restorable point-in-time DB images without requiring a quiet shutdown.
- Occasional short stalls every N shots / at session boundaries when `VACUUM INTO` runs.
- In-memory / test DBs without a file parent skip snapshots (`snapshot_dir() == None`).

## Related

- Implementation: `db/snapshots.rs`, `Database::vacuum_into` in `db/recovery.rs`
- Plan: [plans/hybrid-vacuum-snapshots.md](../plans/hybrid-vacuum-snapshots.md)
- Roadmap: [roadmap.md](../roadmap.md)
