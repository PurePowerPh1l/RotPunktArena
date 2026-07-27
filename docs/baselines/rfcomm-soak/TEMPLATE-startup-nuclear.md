# Startup-Nuclear Soak — TEMPLATE

Ausfüllen **vor** dem ersten Zähllauf. Roh-JSONL: `logs/rfcomm_connection.jsonl` (Zeilen `event=startup_nuclear`).
Toast/PIN immer aktiv mit **ja/nein** markieren — nicht leer lassen. `visibleToastObserved` im Log bleibt `null`.

## Meta

| Feld | Wert |
|------|------|
| Datum | |
| Tester | |
| Hardware-ID / Gerät | KT RDT … |
| Primary BD_ADDR (KnownTarget) | `____________` (fest für alle Läufe) |
| Windows Version / Build | `winver` → |
| Bluetooth-Adapter / Treiber | Gerätemanager → |
| Branch | `feat/rfcomm-bond-gate` |
| tested product commit | `git rev-parse --short HEAD` → |
| working tree | clean / dirty + excluded files |
| hardware conclusion applies to | `<sha> startup-nuclear slice` |
| Build-Hinweis | nach Commit: SHA; Soak nur dem isolierten Produkt-Slice zuordnen |

## Vor jedem Idle-Start

- [ ] App wirklich beendet (kein Tray/Restprozess)
- [ ] Ziel im gewünschten Schlaf-/Idle-Zustand
- [ ] KnownTarget.btAddr = Primary oben (unverändert)
- [ ] JSONL-Datei nicht gelöscht (Append); optional Kopie nach Block: `logs/startup-nuclear-soak-YYYYMMDD.jsonl`

## Freigaberegel (alle müssen PASS)

- [ ] 20/20 Idle-Starts → linked
- [ ] 0 Toasts / 0 PIN-Dialoge außerhalb bewusster Nutzeraktion
- [ ] 0 zweite Nuclear-Operationen pro Start
- [ ] 10/10 Off-/OoR → ruhig Idle, kein Retry
- [ ] 10/10 Race/Exit → kein Zombie, kein Hook-Leak, keine fremde Bond-Änderung
- [ ] 10/10 Link-Lost → Idle, nie Auto-Nuclear
- [ ] Langlauf ≥4h → kein neues Pair/Reconnect, Socket+ENQ halten, keine steigende Fehlerrate

**Bei Fail:** nicht mitten im Soak patchen. Lauf + Log + UI sichern; Muster erst nach dem Block gruppieren.

---

## 0. Smoke (nicht gezählt) — 2–3 Läufe

Prüfen: `forgetScope=primaryOnly`, genau ein Nuclear, `retryScheduled=false`, Hook-Deregistrierung, `linked` plausibel.

| Lauf | runId | Ergebnis | Dauer | Toast/PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-----------|----------|------------|-----------|
| SM-1 | | | | ja / nein | | | |
| SM-2 | | | | ja / nein | | | |
| SM-3 | | | | ja / nein | | | |

Smoke OK? ☐ ja → Zählung starten ☐ nein → stop, Logs sichern

---

## 1. Idle-Starts (20) — Ziel schläft/Idle

Pass: genau 1× `startupNuclear`, `linked=true`, Toast/PIN = **nein**, kein zweiter Forget/Pair.

| Lauf | runId | Ergebnis | Dauer | Toast/PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-----------|----------|------------|-----------|
| S-01 | | | | ja / nein | | | |
| S-02 | | | | ja / nein | | | |
| S-03 | | | | ja / nein | | | |
| S-04 | | | | ja / nein | | | |
| S-05 | | | | ja / nein | | | |
| S-06 | | | | ja / nein | | | |
| S-07 | | | | ja / nein | | | |
| S-08 | | | | ja / nein | | | |
| S-09 | | | | ja / nein | | | |
| S-10 | | | | ja / nein | | | |
| S-11 | | | | ja / nein | | | |
| S-12 | | | | ja / nein | | | |
| S-13 | | | | ja / nein | | | |
| S-14 | | | | ja / nein | | | |
| S-15 | | | | ja / nein | | | |
| S-16 | | | | ja / nein | | | |
| S-17 | | | | ja / nein | | | |
| S-18 | | | | ja / nein | | | |
| S-19 | | | | ja / nein | | | |
| S-20 | | | | ja / nein | | | |

Block 1: ____ / 20 PASS

---

## 1b. Optional: Ziel eingeschaltet (20)

Gleiche Pass-Kriterien; getrennt notieren falls parallel gefahren.

| Lauf | runId | Ergebnis | Dauer | Toast/PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-----------|----------|------------|-----------|
| ON-01 … ON-20 | | | | ja / nein | | | |

Block 1b: ____ / 20 PASS (falls ausgeführt)

---

## 2. Fehlstarts (10) — Ziel aus / außer Reichweite

Pass: ein Nuclear, dann Idle; kein Retry, Toast = **nein**, kein späteres Aufwachen ohne Nutzer.

| Lauf | runId | Ergebnis | Dauer | Toast/PIN | Nuclear# | failedStep | Bemerkung |
|------|-------|----------|-------|-----------|----------|------------|-----------|
| F-01 | | | | ja / nein | | | |
| F-02 | | | | ja / nein | | | |
| F-03 | | | | ja / nein | | | |
| F-04 | | | | ja / nein | | | |
| F-05 | | | | ja / nein | | | |
| F-06 | | | | ja / nein | | | |
| F-07 | | | | ja / nein | | | |
| F-08 | | | | ja / nein | | | |
| F-09 | | | | ja / nein | | | |
| F-10 | | | | ja / nein | | | |

Block 2: ____ / 10 PASS

---

## 3. Race / Exit (10)

Szenarien mischen: Badge während Connecting; Setup öffnen/schließen; App-Exit während Forget / Pair / RFCOMM.
Pass: Attach oder Ignore — nie zweiter Owner-Job; Exit → kein späterer Link; Hook weg; nur Primary-Bond.

| Lauf | Szenario | runId | Ergebnis | Toast/PIN | Nuclear# | Hook OK | Bemerkung |
|------|----------|-------|----------|-----------|----------|---------|-----------|
| R-01 | Badge während Connecting | | | ja / nein | | ja/nein | |
| R-02 | Badge während Connecting | | | ja / nein | | ja/nein | |
| R-03 | Setup open/close während Connecting | | | ja / nein | | ja/nein | |
| R-04 | Exit während Forget | | | ja / nein | | ja/nein | |
| R-05 | Exit während Forget | | | ja / nein | | ja/nein | |
| R-06 | Exit während Pair | | | ja / nein | | ja/nein | |
| R-07 | Exit während Pair | | | ja / nein | | ja/nein | |
| R-08 | Exit während RFCOMM | | | ja / nein | | ja/nein | |
| R-09 | Exit während RFCOMM | | | ja / nein | | ja/nein | |
| R-10 | (Wiederholung kritischstes) | | | ja / nein | | ja/nein | |

Block 3: ____ / 10 PASS

---

## 4. Link-Lost nach Startup-Link (10)

Pass: einmal Idle; **keine** Auto-Nuclear-Folgeaktion.

| Lauf | runId | Idle? | Auto-Nuclear? | Toast/PIN | Bemerkung |
|------|-------|-------|---------------|-----------|-----------|
| L-01 | | ja/nein | ja/nein | ja / nein | |
| L-02 | | ja/nein | ja/nein | ja / nein | |
| L-03 | | ja/nein | ja/nein | ja / nein | |
| L-04 | | ja/nein | ja/nein | ja / nein | |
| L-05 | | ja/nein | ja/nein | ja / nein | |
| L-06 | | ja/nein | ja/nein | ja / nein | |
| L-07 | | ja/nein | ja/nein | ja / nein | |
| L-08 | | ja/nein | ja/nein | ja / nein | |
| L-09 | | ja/nein | ja/nein | ja / nein | |
| L-10 | | ja/nein | ja/nein | ja / nein | |

Block 4: ____ / 10 PASS

---

## 5. Langlauf (≥ 4 h)

Nach erfolgreichem Startup-Link: Live-Session wiederholt starten/stoppen. Socket + ENQ halten; **kein** neues Pair/Connect.

| Feld | Wert |
|------|------|
| Start (Zeit) | |
| Ende (Zeit) | |
| Session start/stop Zyklen | |
| Socket/ENQ gehalten? | ja / nein |
| Neue Pair-/Connect-Versuche? | ja / nein (JSONL prüfen) |
| Fehlerrate steigend? | ja / nein |
| Bemerkung | |

Block 5: ☐ PASS ☐ FAIL

---

## Mehrere RedDot-Bonds (5) — optional parallel / nach Block 1

Nur Primary-BD_ADDR darf vergessen/neu gekoppelt werden. Windows-Einstellungen vor/nach vergleichen.

| Lauf | runId | Fremde Bonds unverändert? | Bemerkung |
|------|-------|---------------------------|-----------|
| M-01 | | ja / nein | |
| M-02 | | ja / nein | |
| M-03 | | ja / nein | |
| M-04 | | ja / nein | |
| M-05 | | ja / nein | |

---

## Ergebnis / Abnahmevermerk (nach PASS)

```text
Hardware: …
Adapter: …
Windows: …
Datum: …
Läufe: Smoke + 20 Idle + 10 Fail + 10 Race + 10 LinkLost + Langlauf
Resultat: PASS
JSONL-Archiv: logs/… bzw. docs/baselines/rfcomm-soak/…
Commit danach: Startup Nuclear; Soft-A/B-Produktreste löschen; bt_auto_ab = Diagnose-only
```

Kopieren nach: `docs/baselines/rfcomm-soak/YYYY-MM-DD-startup-nuclear-<host>.md`
