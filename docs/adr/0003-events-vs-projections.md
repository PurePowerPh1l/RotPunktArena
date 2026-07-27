# ADR: Events vs Projections

Status: Accepted (Welle A)  
Date: 2026-07-22

## Context

Scores must be rebuildable and auditable. Storing only UI state loses provenance.

## Decision

- **`frames`**: immutable raw evidence (`raw_frame_hex`, `frame_sha256`, `parser_version`, `parse_status`).
- **`events`**: append-only log with `UNIQUE(session_id, sequence)`, `actor_type`, optional `parser_version`.
- **`shots`**: rebuildable projection keyed by `frame_id` UNIQUE + `session_sequence`.
- Live UI reads in-memory snapshot fed from Accepted ingest; SQLite remains source of truth.

## Competition lifecycle (later)

Competition / entries / people are **setup entities** (Büro). They do not replace the event log. Linking a live session to `entry_id` is optional metadata; scoring integrity stays frame→event→projection regardless of Wettkampf status (`draft` / `active` / `closed`).

## Consequences

- Projections can be rebuilt from events + frames if corrupted.
- No “Speichern” button — accepting a frame *is* the commit.
