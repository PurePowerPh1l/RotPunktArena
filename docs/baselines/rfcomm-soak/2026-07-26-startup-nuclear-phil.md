# Startup-Nuclear Soak — 2026-07-26 (Phil)

Roh-JSONL: `logs/rfcomm_connection.jsonl` (`event=startup_nuclear`).
Toast/PIN aktiv **ja/nein**. `visibleToastObserved` im Log = `null`.

## Meta

| Feld | Wert |
|------|------|
| Datum | 2026-07-26 |
| Tester | Phil |
| Hardware-ID / Gerät | KT RDT ZIE 1 S/N 203 |
| Primary BD_ADDR (KnownTarget) | `0018DA070564` (KT RDT ZIE 1 S/N 203) |
| Windows Version / Build | Windows 10 Pro 25H2, Build **26200.8875** |
| Bluetooth-Adapter / Treiber | Realtek Bluetooth LEAI Driver (`USB\VID_0BDA&PID_8922\00E04C885A01`) — Treiberversion im Gerätemanager ergänzen |
| Branch | `feat/rfcomm-bond-gate` |
| tested product commit | `5b1dcd3` |
| working tree | additionally dirty; excluded: WakeSheet, LiveStandView, App.css, `bt_start_pair_variants`+script, `discovery.rs` Pair-Report, `.cursor/rules/einfach-genial.mdc` |
| hardware conclusion applies to | `5b1dcd3` startup-nuclear slice |
| Build-Hinweis | Smoke via `bt_cold_start` (= `ConnectionManager::start` → Startup Nuclear), nicht Tauri-UI. Soak lief vor dem Commit auf dem später isolierten Slice; Inhalt von `5b1dcd3` = getesteter Produktpfad (Owner/Nuclear/setup/diag). |

## Smoke-Ergebnis (2026-07-26 ~02:01–02:02 UTC)

Artefakt: `bt_cold_start` (Owner-Startpfad identisch zur App). JSONL: `logs/rfcomm_connection.jsonl`.

| Lauf | runId | Ergebnis | Dauer | Toast | PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-------|-----|----------|------------|-----------|
| SM-1 | `r20260726T020107-g1` | linked | 7.5 s | **nein** | **nein** | 1 | – | Pflichtfelder JSONL OK; `authCallbackCount=0` |
| SM-2 | `r20260726T020132-g1` | linked | 6.6 s | **nein** | **nein** | 1 | – | ditto |
| SM-3 | `r20260726T020157-g1` | linked | 7.1 s | **nein** | **nein** | 1 | – | ditto |

JSONL-Pflichtfelder (alle drei): `origin=startupNuclear`, `forgetScope=primaryOnly`, `forgetResult=ok`, `pairResult=ok`, `rfcommResult=ok`, `linked=true`, `retryScheduled=false`, `hookDeregistered=true`.

Smoke OK? ☑ **ja** → S-01 gestartet · Toast/PIN vom Operator bestätigt (2026-07-26)

---

## 1. Idle-Starts (20) — Ziel Idle / schlafend

Pass: genau 1× `startupNuclear`, `linked=true`, Toast/PIN = **nein**, kein zweiter Forget/Pair.
Artefakt unverändert: `bt_cold_start` (= Owner-Start → Startup Nuclear, Commit `5b1dcd3`).

JSONL-Archiv Block: `logs/startup-nuclear-S-block.json`

| Lauf | runId | Ergebnis | Dauer | Toast | PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-------|-----|----------|------------|-----------|
| S-01 | `r20260726T020638-g1` | linked | 7.2 s | nein* | nein* | 1 | – | |
| S-02 | `r20260726T020705-g1` | linked | 7.4 s | nein* | nein* | 1 | – | |
| S-03 | `r20260726T020719-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-04 | `r20260726T020729-g1` | linked | 4.9 s | nein* | nein* | 1 | – | |
| S-05 | `r20260726T020740-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-06 | `r20260726T020750-g1` | linked | 3.5 s | nein* | nein* | 1 | – | |
| S-07 | `r20260726T020800-g1` | linked | 6.1 s | nein* | nein* | 1 | – | |
| S-08 | `r20260726T020812-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-09 | `r20260726T020822-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-10 | `r20260726T020833-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-11 | `r20260726T020843-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-12 | `r20260726T020854-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-13 | `r20260726T020905-g1` | linked | 4.9 s | nein* | nein* | 1 | – | |
| S-14 | `r20260726T020915-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-15 | `r20260726T020926-g1` | linked | 3.6 s | nein* | nein* | 1 | – | |
| S-16 | `r20260726T020935-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-17 | `r20260726T020946-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-18 | `r20260726T020956-g1` | linked | 3.6 s | nein* | nein* | 1 | – | |
| S-19 | `r20260726T021006-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |
| S-20 | `r20260726T021016-g1` | linked | 4.8 s | nein* | nein* | 1 | – | |

\*Toast/PIN: Operator bestätigt für Smoke **und** S-Block (nein/nein), 2026-07-26.

Block 1: **20 / 20 PASS** (JSONL: origin/primaryOnly/linked/retry=false/hook=true, je 1 Nuclear)

---

## 2. Fehlstarts (10) — Ziel aus / außer Reichweite

Pass: ein Nuclear, dann Idle; kein Retry; Toast/PIN = nein; kein späteres Aufwachen ohne Nutzer.
JSONL-Archiv: `logs/startup-nuclear-F-block.json`

| Lauf | runId | Ergebnis | Dauer | Toast | PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-------|-----|----------|------------|-----------|
| F-01 | `r20260726T021311-g1` | idle | 21.7 s | nein* | nein* | 1 | pair | `retryScheduled=false`; `hookDeregistered=false` (Fail vor RFCOMM-Guard — Pair-RAII uninstallt trotzdem) |
| F-02 | `r20260726T021339-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-03 | `r20260726T021408-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-04 | `r20260726T021436-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-05 | `r20260726T021504-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-06 | `r20260726T021533-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-07 | `r20260726T021601-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-08 | `r20260726T021630-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |
| F-09 | `r20260726T021658-g1` | idle | 21.7 s | nein* | nein* | 1 | pair | ditto |
| F-10 | `r20260726T021727-g1` | idle | 21.6 s | nein* | nein* | 1 | pair | ditto |

\*Toast/PIN: Operator bestätigt F-Block (nein/nein), 2026-07-26. RedDot danach wieder an.

Block 2: **10 / 10 PASS** (idle, kein Retry, je 1 Nuclear, `failedStep=pair`)

---

## 3. Race / Exit (10)

Harness: `bt_startup_race` (Lab only; Owner/Nuclear unverändert). JSONL: `logs/startup-nuclear-R-block.json`

| Lauf | Szenario | runId | Ergebnis | Toast | PIN | Nuclear# | Hook OK | Bemerkung |
|------|----------|-------|----------|-------|-----|----------|---------|-----------|
| R-01 | Badge während Connecting | `r20260726T022029-g1` | linked | nein* | nein* | 1 | ja | Attach Ok; gen 1→1; kein 2. Nuclear |
| R-02 | Badge während Connecting | `r20260726T022053-g1` | linked | nein* | nein* | 1 | ja | ditto |
| R-03 | Setup Pause während Connecting | `r20260726T022115-g1` | idle/cancelled | nein* | nein* | 1 | ja | `failedStep=cancelled`; kein Linked |
| R-04 | Exit während Forget | _(kein startup_nuclear-Log bei Shutdown-Return)_ | Shutdown in „vorbereitet“ | nein* | nein* | 0 neu | – | kein late Linked (+8s) |
| R-05 | Exit während Forget | ditto | ditto | nein* | nein* | 0 neu | – | ditto |
| R-06 | Exit während Pair | ditto | Shutdown in „Kopple“ | nein* | nein* | 0 neu | – | kein late Linked |
| R-07 | Exit während Pair | ditto | ditto | nein* | nein* | 0 neu | – | ditto |
| R-08 | Exit während RFCOMM | `r20260726T022234-g1` | idle (STOP) | nein | nein | 1 | – | Phase „Verbinde“ nicht erreicht; `failedStep=pair` Win32 31; **gezählt, nicht überschrieben** |
| R-08b | Exit während RFCOMM (nach Settle) | _(kein neues startup_nuclear)_ | Shutdown in „Verbinde“ | nein* | nein* | 0 neu | – | linkedDelta=0 nach +8s |
| R-09 | Exit während RFCOMM | ditto | Shutdown in „Verbinde“ | nein* | nein* | 0 neu | – | linkedDelta=0 |
| R-10 | Badge während Connecting | `r20260726T022529-g1` | linked | nein* | nein* | 1 | ja | Attach Ok; gen 1→1 |

\*Toast/PIN: Operator bestätigt (nein/nein), 2026-07-26.

Block 3: **PASS mit dokumentiertem R-08-Negativ** (Pair/Win32 31 nach Exit-Pair-Stress). R-01–R-07, R-08b, R-09, R-10 grün; kein Toast/PIN, kein Zombie-Link, Badge-Attach single-flight ok.

---

## 4. Link-Lost nach Startup-Link (10)

Pass: einmal Idle; **keine** Auto-Nuclear-Folgeaktion. Harness: `bt_startup_race link_lost`.
Operator: Block vorzeitig beendet (2026-07-26) — genug Evidenz für Idle ohne Auto-Nuclear.

| Lauf | Ergebnis | Auto-Nuclear? | Toast | PIN | Bemerkung |
|------|----------|---------------|-------|-----|-----------|
| L-01 | **PASS** Idle ~37 s | nein | nein | nein | Winsock 10053 |
| L-02 | STOP (120 s linked) | – | – | – | Gerät nicht aus; nicht überschrieben |
| L-03 | **PASS** | nein | nein | nein | |
| L-04 | **PASS** | nein | nein | nein | |
| L-05 | **PASS** | nein | nein | nein | |
| L-06 | STOP (nie Linked) | – | – | – | Start bei Gerät aus |
| L-06b | **PASS** | nein | nein | nein | Nachholen |
| L-07 | STOP (nie Linked) | – | – | – | Start bei Gerät aus |
| L-07b | **PASS** | nein | nein | nein | Nachholen |
| L-08…L-10 | **nicht gefahren** | | | | Operator: Block reicht |

Block 4: **6× PASS Link-Lost → Idle, 0× Auto-Nuclear** (bei erfolgreichen Läufen). 3× Operator-/Timing-STOP dokumentiert. Kein Gegenbeispiel für Auto-Nuclear. Operator: Block vorzeitig beendet.

---

## 5. Langlauf (≥ 4 h)

| Feld | Wert |
|------|------|
| Start | `2026-07-26T04:52:21+02:00` |
| Erwartetes Ende | `2026-07-26T08:52:21+02:00` |
| Tatsächliches Ende | PASS im Log (`PASS long_hold 14400s`) |
| Hold | 14400 s |
| Harness | `scripts/run-startup-nuclear-long-hold.ps1 -Detach` |
| PID | 34156 (beendet) |
| Log | `logs/long-hold-4h.err.log` (Fortschritt/PASS) |
| Session-Zyklen | **120** (RegisterSink/UnregisterSink) |
| Socket/ENQ gehalten? | **ja** — durchgängig `status=linked`, gen 1→1 |
| Neue Pair-/Connect? | Baseline nuc 49 → 51 (+1 Start-Nuclear des Holds; kein weiteres während Hold) |
| link_lost während Hold | **0** (Baseline 10 → 10) |
| Bemerkung | Alle 15-min-Progress-Zeilen `status=linked` |

Block 5: ☑ **PASS**

## Commit-Grenze (nach Freigabe, nicht jetzt)

## Commit-Grenze (nach Freigabe, nicht jetzt)

## Commit-Grenze (nach Freigabe, nicht jetzt)

**Aussage des Produkt-Commits:** Known Target wird beim Start einmal Primary-only Nuclear repariert; keine konkurrierende Operation entsteht.

### In den Startup-Nuclear-Commit

| Pfad | Warum |
|------|--------|
| `apps/desktop/src-tauri/src/connection/owner.rs` | startupNuclear, Fail→Idle, Single-Flight |
| `apps/desktop/src-tauri/src/connection/nuclear.rs` | PrimaryOnly / Report |
| `apps/desktop/src-tauri/src/connection/setup_flow.rs` | Attach während Connecting |
| `apps/desktop/src-tauri/src/connection/diag.rs` | `startup_nuclear` + `runId` |
| `apps/desktop/src-tauri/src/connection/connect_policy.rs` | Origin `startupNuclear` |
| `apps/desktop/src-tauri/src/connection/mod.rs` / `manager.rs` / `generation_tests.rs` | Kommentare / Policy-Bezug |
| `apps/desktop/src-tauri/src/commands/live.rs` | Origin-Doku |
| `apps/desktop/src-tauri/src/transport/rfcomm/auth_hook.rs` | `authCallbackCount` |
| `apps/desktop/src-tauri/src/bin/bt_nuclear_smoke.rs` | Skip-Env-Kommentar |
| `docs/bluetooth-connection-stack.md` | Startup Nuclear Vertrag |
| `docs/rfcomm-nuclear-test-matrix.md` | Soak-Reihenfolge / Freigabe |
| `docs/baselines/rfcomm-soak/TEMPLATE-startup-nuclear.md` | Arbeitsblatt |
| diese Datei (ausgefüllt) + gesichertes JSONL | Evidenz |

### Separat (eigener Commit / nicht Freigabe-Slice)

| Pfad | Warum |
|------|--------|
| `RedDotWakeSheet.tsx`, `LiveStandView.tsx`, `App.css` | UI, getrennt vom Auto-Start-Repair |
| `bt_start_pair_variants.rs`, `scripts/run-start-pair-variants.ps1`, ggf. `Cargo.toml`-Bin | Lab-Artefakt |
| `discovery.rs` Pair-Report | Produkt-Nuclear nutzt ihn **nicht** → Diagnose-Commit |
| `.cursor/rules/einfach-genial.mdc` | Rule, nicht Produktpfad |

## Smoke-Gate (vor S-01)

Pro Lauf müssen **alle** Felder stimmen:

```text
origin              = startupNuclear
forgetScope         = primaryOnly
forgetResult        = ok
pairResult          = ok
rfcommResult        = ok
linked              = true
retryScheduled      = false
hookDeregistered    = true
Nuclear operations  = 1
Toast / PIN dialog  = nein / nein
```

`authCallbackCount = 0` allein = kein Fail.

| Lauf | runId | Ergebnis | Dauer | Toast | PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-------|-----|----------|------------|-----------|
| SM-1 | | | | ja/nein | ja/nein | | | |
| SM-2 | | | | ja/nein | ja/nein | | | |
| SM-3 | | | | ja/nein | ja/nein | | | |

Smoke OK? ☐ ja → S-01 starten · ☐ nein → stop, Logs sichern, Muster aus `failedStep` / Dauern / Winsock

## Während des Soaks

- Keine Patches / Rebuilds / Config-Wechsel innerhalb eines Blocks.
- Fail = zählen + dokumentieren; nicht überschreiben.
- Toast, PIN, Doppel-Nuclear, fremder Bond, Zombie → Block stoppen, Logs + Zeitpunkt.
- Nach Exit Prozessende prüfen.
- Badge-Test **während** Connecting (nicht nach Linked).

---

*(Blöcke S/F/R/L/Langlauf: Zeilen aus `TEMPLATE-startup-nuclear.md` hierher kopieren, sobald Smoke grün.)*
