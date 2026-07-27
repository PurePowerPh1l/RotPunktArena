# RedDot Arena — Phasen-Roadmap (Gap-Analyse)

Stand: Schema **v9**, App als Phase-1-MVP.  
Status: **DONE** / **PARTIAL** / **MISSING**. Abgleich der überarbeiteten Phasen-Checkliste gegen den Code.

Verwandt: [reddot-arena-review.md](./reddot-arena-review.md) · [ai-review-checklist.md](./ai-review-checklist.md) · Implementierungsplan Snapshots: [plans/hybrid-vacuum-snapshots.md](./plans/hybrid-vacuum-snapshots.md)

---

## Phase 0 — Protokoll

| Item | Status | Kurz |
|---|---|---|
| Herkunft / Validierungsstatus | DONE | [`protocol/provenance.md`](./protocol/provenance.md) |
| [`protocol.md`](./protocol.md) | DONE | ENQ/NAK/STX, 59-Byte-Frame (Geräteverhalten) |
| Parser TS + Rust | DONE | [`packages/protocol`](../packages/protocol), [`protocol.rs`](../apps/desktop/src-tauri/src/protocol.rs) |
| Sniffer Mock/Replay | DONE | [`tools/sniffer`](../tools/sniffer) |
| Live-Captures echte Hardware | MISSING | Workflow/Checkliste/Auto-`--out` bereit ([captures/CHECKLIST.md](./captures/CHECKLIST.md)); Captures selbst fehlen |
| Header `[1..31]` / Trailer `[55..58]` | MISSING | `sniffer analyze` vorbereitet; Mapping wartet auf Live-Daten |
| Baudrate / schnelle Schussfolge | PARTIAL | 9600 in Code; Live **pending validation**; Burst-Session in Checkliste Session C |
| Reconnect-Verhalten dokumentiert | PARTIAL | Softwareskizze in [`protocol.md`](./protocol.md); Live **pending validation** |
| Golden-Fixtures aus echten Daten | MISSING | nur [`captures/synthetic-shot.hex`](./captures/synthetic-shot.hex); Templates unter `captures/live/` |

**Blocker:** echte Hardware-Captures.

---

## Phase 1 — Stand-MVP

### Kern (DONE)

Transport Sim/Serial/Replay, Single Writer, Dedupe, Event-Log, Live-UI, Integrationstests, ADRs 0001–0003, Autosave-Marker in Ingest-TX, Recovery-Gate + Aktionen, Commands-Modularisierung, EventKind TS↔Rust angeglichen, `PRAGMA journal_mode=WAL; synchronous=NORMAL` in [`db/mod.rs`](../apps/desktop/src-tauri/src/db/mod.rs).

### Präzisierung Snapshot / Export

| Item | Status | Kurz |
|---|---|---|
| `VACUUM INTO` (nicht Dateikopie) | DONE | [`db/recovery.rs`](../apps/desktop/src-tauri/src/db/recovery.rs) + Session-/Cadence-Trigger in [`db/snapshots.rs`](../apps/desktop/src-tauri/src/db/snapshots.rs); ADR [0004](./adr/0004-wal-safe-snapshots.md) |
| Snapshot-Trigger nach N Schüssen | DONE | alle 100 Accepted-Schüsse (`SNAPSHOT_EVERY_N_SHOTS`), außerhalb der Ingest-TX |
| Hybrid Session-Grenzen + Event-Count | DONE | Start/Ende in `sessions.rs` + Cadence nach Ingest-COMMIT |
| Notfall-ZIP vollständig | PARTIAL | ZIP: `reddot.sqlite` + `manifest.json` + optional `events.jsonl`. **Fehlt separat:** Rohframes-Datei, Checksums-Datei (Frames nur indirekt in DB) |
| `synchronous=NORMAL` dokumentiert | DONE | Code + [ADR 0004](./adr/0004-wal-safe-snapshots.md) (inkl. File-Copy-Warnung) |
| Produktionsmodus-Schalter | PARTIAL | Dev-Panel klappbar, Aim nur bei Dev; Toolbar-Dev-Button immer sichtbar, kein persistenter Prod-Flag |

**Umsetzung:** [plans/hybrid-vacuum-snapshots.md](./plans/hybrid-vacuum-snapshots.md) · [ADR 0004](./adr/0004-wal-safe-snapshots.md)

---

## Phase 2 — Wettkampf

| Item | Status | Kurz |
|---|---|---|
| Personen / Wettkampf / Startliste | DONE | Büro-Panels + Schema v2 |
| Nachkauf v7: volle Serien + Best-of | DONE | Checkbox; nach `done` neue Serie; Limit = `max_shots`/Session; Ranking beste Serie |
| Mannschaften v8, DnD, Teamwertung | DONE | DnD + `list_team_results` im Büro |
| Training-Historie + Auto-Save | DONE | bei `end_session` |
| Ergebnisübersicht + Detail/Druck | DONE | HTML/`window.print` (kein natives PDF) |
| Lifecycle `draft→running→provisional→official→archived` | MISSING | Ist: `draft\|active\|closed` |
| `ResultStatus` live/complete/…/dns/dnf/dq | MISSING | Ist Entry: `waiting\|probe\|active\|done` |
| `RuleProfile` versioniert einfrieren | MISSING | nur `ScoringMode` an Competition |
| Korrekturen Begründung + Actor | MISSING | `actor_type` am Event-Log, kein Korrektur-Workflow |
| Team-Umbenennen-UI | PARTIAL | Command/API da; UI in `TeamsPanel` fehlt |
| Teamwertung live beim Schießen | PARTIAL | nur Büro-Refresh, nicht in Live-UI |
| Bulk-Aktionen Startliste | PARTIAL | nur Clone/„Übernehmen“; kein Multi-Select |
| Export Training CSV/PDF | MISSING | nur Listen-API |
| `ResultSnapshot`-Tabelle | MISSING | keine Migration/Tabelle |

---

## Phase 3 — Vereinsserver (optional)

Alles **MISSING** außer Domänen-Vorbereitung in [`packages/domain`](../packages/domain): Sync-API, zentrale Stammdaten, Zuschauer-Web, Rollen, Datenschutz-Profile, LAN-PIN, [`packages/contracts`](../packages/contracts) (Ordner existiert nicht; Guidelines verlangen ihn bereits).

---

## Phase T — Ethernet

| Item | Status |
|---|---|
| Hardware-Machbarkeit | MISSING |
| Serial↔Ethernet-Bridge | MISSING (Skizze in [`transport.md`](./transport.md)) |
| `TcpTransport` | MISSING (`TransportKind::Tcp` Stub) |
| iPad/Android-Hinweise | MISSING in Docs |

---

## Querschnitt

| Item | Status |
|---|---|
| [`ai-review-checklist.md`](./ai-review-checklist.md) | DONE |
| Ein Schreibpfad / Aggregat | DONE (Regel + Ingest-Kern) |
| Migrationen eine Änderung/Version | DONE (v1–v9) |
| Keine God-Files (~300 Z.) | PARTIAL (`competitions.rs` ~631, `session_lifecycle` ~369, …) |
| Naming Rust↔TS enforced | PARTIAL (Konvention ja, kein Gate) |
| Business-Logik nur Rust | PARTIAL (Training-Stats/XP in TS) |

---

## Empfohlene Priorität

1. **Hardware-Captures** (Phase-0-Blocker für Header/Trailer/Fixtures)
2. **Hybrid-`VACUUM INTO`-Snapshots** + WAL/File-Copy-Warnung in Docs → [plans/hybrid-vacuum-snapshots.md](./plans/hybrid-vacuum-snapshots.md) **(DONE)**
3. **Phase-2 Quick Wins:** Team-Rename-UI, Live-Teamwertung, Prod-Schalter
4. **Phase-2 Domain-Erweiterung:** Lifecycle, ResultStatus, RuleProfile-Freeze
5. **Publishing/Audit:** Korrekturen, ResultSnapshot, Training-Export
6. Phase 3 / T bewusst später

**Phase 0 ohne Hardware:** Capture-Workflow vorbereitet unter [`captures/`](./captures/) (Live-Ordner, Checkliste, Sniffer `--out` / `analyze`).
