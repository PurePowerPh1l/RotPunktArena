# ADR: Shot Integrity (atomic persist before UI)

Status: Accepted (Welle A)  
Date: 2026-07-22

## Context

UI freezes and double-counting risked silent loss or inflation of scores. Local-first RedDot Arena must treat every scored shot as durable and idempotent.

## Decision

1. Persist the **raw frame** (hex + SHA-256) in SQLite **before** any UI emission.
2. Ingest is one **IMMEDIATE transaction**: frame → dedupe → parse → `shot_received` event → `shots` projection → `sessions.next_sequence++` → COMMIT.
3. UI/emit only on `IngestOutcome::Accepted` after COMMIT (**fail-closed**).
4. Dedupe: `device_sequence` if present; else `SHA-256(raw)`; empty-frame fingerprint only as last resort.
5. `parser_version` (`reddot-stx-v1`) stored on frame and shot events.

## Consequences

- Duplicate device retries do not create extra scores.
- Crash after COMMIT still shows the shot once on reopen.
- Crash before COMMIT → no UI shot (device may resend).
