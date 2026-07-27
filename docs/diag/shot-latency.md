# DIAGNOSE-ONLY — Shot-Latenz-Telemetrie

Beobachtet die Registrierungslatenz echter Schüsse in einer **bereits laufenden** Serie.
Ändert kein Produktverhalten (ENQ-Takt, Fanout, Epoch, Pause-ACK, Arena-Gate, UI-Contract,
`wait_timeout(80)`-Dauer).

**Standard: AUS.** Ohne Aktivierung kein Writer-Thread, keine JSONL-Datei, keine
pro-Shot-Serialisierung, keine Poll-Loop-Trace-Arbeit.

## Aktivierung

```powershell
$env:REDDOT_SHOT_LATENCY_DIAG = "1"
# App starten (Desktop / cargo run …)
```

Akzeptierte Werte: `1`, `true`, `yes` (case-insensitive). Jeder andere Wert / fehlende Variable = aus.

## Log-Datei

`logs/shot_latency.jsonl` (Repo-Root `logs/`, gleiches Layout wie `rfcomm_connection.jsonl`).

Eine JSON-Zeile **nur** bei Arena-`Accepted` + UI-`emit("shot")` mit RFCOMM-Provenance
(und nur wenn Diagnose aktiv). **Keine** Zeile pro Poll-Iteration.

## Architektur (Poll blockiert nicht auf Datei-I/O)

```text
Poll (nach emit)
  → baut ShotLatencyRecordOwned
  → try_send → sync_channel(32)
       Full/Disconnected → Drop + Counter, kein Block
Diagnose-Writer-Thread (nur wenn aktiv)
  → BufWriter<File>, eine Datei für den Lauf
  → writeln + flush() best-effort pro Zeile (sichtbar ohne App-Exit)
  → kein sync_all
```

## Poll-Loop-Trace (Schema v2) — Akkumulation

```text
iteration_id++   (nur wenn DIAG an)
was_pending = open incomplete frame?
read (50 ms, unverändert)
  DIAG an  → stamp start/end/result
  DIAG aus → keine Instant-/Klassifikationsarbeit
if was_pending: count read in intervening (auch Completing-Read)
feed parser
  complete → close frame (kein späterer Wait/Read für diesen Shot)
  still pending → nachfolgender wait_timeout(80) zählt zu pollInterveningWaitMs
```

`pollReadResult` beschreibt **nur** den First-Chunk-Read (bei Accepted RFCOMM meist `bytes`).
Fragment-Hypothese: `pollInterveningWaitMs` / `pollIntervening*Reads`.

### Entscheidungsregel (historisch → erledigt)

Vor dem Idle-Wait-Skip zeigte Competition-v2: Split-Frames mit
`pollInterveningWaitMs ≈ 80` und bridge→parser-Cluster 80–95 ms.
**Produkt-Fix:** Bei RFCOMM wird `wait_timeout(80)` übersprungen, solange
`parser.has_incomplete_shot_frame()` nach dem Feed true ist.

`FrameProvenanceTracker::discard` ist eine **Tracker-API für Tests / künftige Hooks**.
Es gibt derzeit **keinen** Poll-Pfad, der den Parser mid-session resettet und discard
aufruft — kein verdrahtetes Produkt-Discard/Resync.

### Hardware acceptance — DONE

`fix(poll): skip idle wait while rfcomm frame is incomplete` (`dc66be6`)

```text
PASS — Competition, n=20, schema v2
runId: shotlat-20260726T181633.776
- Split frames: 15/20
- Split pollInterveningWaitMs: 0 for all split samples
- No 80–95 ms bridge→parser idle-wait cluster
- app_rx→ui substantially improved (p50 ~95→63 ms, p95 ~171→78 ms)
- sink full/disconnected: 0
- JSONL order valid
```

Restlatenz einzelner ~50-ms-Werte bei Empty-Reads = unveränderter
`read_timeout(50)` — kein Anlass für einen weiteren Poll-Fix.

## Messbefehl (Hardware)

1. `REDDOT_SHOT_LATENCY_DIAG=1`, App starten.
2. BT linked, Competition (Hardware) starten.
3. ≥20 echte Schüsse ohne Serienwechsel.
4. Auswerten:

```powershell
Get-Content .\logs\shot_latency.jsonl | Select-Object -Last 30
```

Offline-Deltas:

- `app_rx_to_ui_ms` ≈ `shotEventEmittedOffsetMs - ownerRxOffsetMs`
- `bridge_to_parser_ms` ≈ `pollParserFrameOffsetMs - bridgeReceivedOffsetMs`
- `enq_to_ui_ms` ≈ `shotEventEmittedOffsetMs - lastEnqSentOffsetMs` (wenn gesetzt)
- `sink_to_bridge_ms` ≈ `bridgeReceivedOffsetMs - sinkEnqueuedOffsetMs`

## Schema v2 (camelCase JSON)

```json
{
  "schemaVersion": 2,
  "runId": "shotlat-20260726T154712.123",
  "wallTs": "2026-07-26T15:47:12.456Z",
  "sessionId": "…",
  "mode": "competition",
  "shotTraceId": "session-uuid:a1b2c3d4e5f67890",
  "firstChunkRxSeq": 42,
  "ownerRxOffsetMs": 12040,
  "lastEnqSentOffsetMs": 11800,
  "sinkEnqueuedOffsetMs": 12040,
  "bridgeReceivedOffsetMs": 12110,
  "bridgeFirstChunkReceivedOffsetMs": 12110,
  "pollParserFrameOffsetMs": 12200,
  "ingestStartedOffsetMs": 12201,
  "sqliteCommittedOffsetMs": 12202,
  "shotEventEmittedOffsetMs": 12203,
  "sinkTrySendOkCount": 42,
  "sinkTrySendFullCount": 0,
  "sinkTrySendDisconnectedCount": 0,
  "bridgeTryRecvCount": 50,
  "staleEpochDropCount": 0,
  "pollIterationId": 10,
  "pollWaitStartedOffsetMs": 12020,
  "pollWaitReturnedOffsetMs": 12100,
  "pollFirstReadStartedOffsetMs": 12105,
  "pollFirstReadReturnedOffsetMs": 12110,
  "pollReadResult": "bytes",
  "pollFrameCompleteIterationId": 11,
  "pollInterveningWaitMs": 80,
  "pollInterveningReadCalls": 1,
  "pollInterveningByteReads": 1,
  "pollInterveningEmptyReads": 0,
  "pollInterveningReadWaitMs": 4
}
```

- Offsets relativ zum Process-`runId`-Anchor (`Instant`), nicht Wall-Clock-Deltas.
- `bridgeFirstChunkReceivedOffsetMs` ist ein **Alias** von `bridgeReceivedOffsetMs`
  (derselbe First-Chunk-Bridge-Zeitpunkt, kein zweiter Messpunkt).
- Keine Rohframes, Scores oder Koordinaten.

## Frontend-Lücke (bewusst)

`UiShot` enthält weder `sessionId` noch `frameSha16` — kein Contract-Change.
Kernmetriken enden bei `shotEventEmittedOffsetMs`.
