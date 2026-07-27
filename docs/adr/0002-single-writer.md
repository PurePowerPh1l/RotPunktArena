# ADR: Single Writer (Rust Arena Core)

Status: Accepted (Welle A)  
Date: 2026-07-22

## Context

SQLite + concurrent UI/worker access previously risked lock contention and inconsistent state. Splitting into range-agent + browser is deferred.

## Decision

- **Only Rust** writes SQLite (Tauri commands + poll worker ingest).
- React is read/subscribe only (commands + events).
- Poll worker opens its **own** DB connection (WAL + `busy_timeout`); engine holds another for bureau/session setup.
- No sync listen handlers that re-lock while emit is in flight.

## Consequences

- Clear ownership of durability.
- Bureau/competition schema may exist but must not become a second writer from the UI.
- Future range-agent split can extract the same Arena Core crate without changing the integrity contract.
- **DB file snapshots** (`VACUUM INTO`, see [ADR 0004](./0004-wal-safe-snapshots.md)) also run only on this Rust writer path — never from the frontend and never via raw copy of the live WAL database.
