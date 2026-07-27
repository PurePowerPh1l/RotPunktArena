# Live-Hardware Capture — Checkliste

Status: **vorbereitet**, wartet auf RedDot-Ziel am COM-Port.  
Ziel: Header `[1..31]` und Trailer `[55..58]` klären + Golden-Fixtures aus echten Daten.

Siehe auch: [README.md](./README.md) · [protocol.md](../protocol.md) · Sniffer `tools/sniffer`

---

## Vor dem Anschluss (einmalig)

- [ ] Node 20+ installiert; in `tools/sniffer`: `npm install`
- [ ] Adapter/Kabel bereit (RS-232 oder Bluetooth-SPP → virtueller COM)
- [ ] RedDotView / andere Software **geschlossen** (Port sonst belegt)
- [ ] Dry-Run ohne Gerät:

  ```powershell
  cd "D:\Disag Reddot 2\tools\sniffer"
  npm run sniffer -- ports
  npm run sniffer -- replay ../../docs/captures/synthetic-shot.hex
  npm run sniffer -- analyze ../../docs/captures/synthetic-shot.hex
  ```

---

## Session A — Idle / Keepalive (kein Schuss)

Ziel: ENQ→NAK-Rhythmus und Reconnect-Baseline.

```powershell
npm run sniffer -- port COMx --duration 20000
```

Erzeugt automatisch `docs/captures/live/live-YYYYMMDD-HHMMSS.hex`.

Danach umbenennen/kopieren nach:

- `docs/captures/live/idle-enq-nak.hex` (oder belassen + in Tabelle unten notieren)

Auswertung:

```powershell
npm run sniffer -- replay ../../docs/captures/live/<datei>.hex
```

- [ ] Datei enthält `TX: 05` und `RX: 15`
- [ ] Poll ~300 ms erkennbar (Timestamps in `#`-Kommentaren / Konsole)
- [ ] Notiz in `protocol.md` Reconnect-Abschnitt, falls Abweichung

---

## Session B — Einzelne Schüsse

Ziel: mindestens **3** vollständige 59-Byte-STX-Frames mit bekannten Treffern (z. B. Mitte, 10er, Rand).

```powershell
npm run sniffer -- port COMx --duration 120000
```

Während der Session: bewusst Schüsse abgeben; nach Session:

```powershell
npm run sniffer -- analyze ../../docs/captures/live/<datei>.hex
```

- [ ] ≥3 `SHOT`-Events im Replay
- [ ] `analyze`: Header-/Trailer-Hex notieren
- [ ] Header über Frames **gleich oder verschieden**? (Konsole gibt Hinweis)
- [ ] Trailer gleich/verschieden?
- [ ] Bekannte Felder value/x/y stimmen mit Scheibe/Anzeige überein
- [ ] Beste Capture → `docs/captures/live/shot-series.hex` (oder mehrere `shot-*.hex`)

---

## Session C — Schnelle Schussfolge (Burst)

Ziel: Baudrate/Parser bei dichter Folge.

```powershell
npm run sniffer -- port COMx --duration 60000
```

- [ ] Keine verlorenen/zusammengeklebten Frames (Replay + Arena-App parallel testen)
- [ ] Capture als `live/burst-rapid.hex` ablegen
- [ ] Beobachtung in `protocol.md` „Offene Punkte“ abhaken oder ergänzen

---

## Session D — GetVars (optional)

Manuell oder später CLI-Erweiterung: `DC1 0F B4` → ACK → Index → 4-Byte-Antwort.

- [ ] Capture `live/getvars.hex`
- [ ] `application_version` / `revision` dokumentieren

---

## Nach den Captures (ohne Gerät)

1. `analyze` auf allen Live-`.hex` → Offsets in [protocol.md](../protocol.md) eintragen
2. Beste echte Frames als **Golden Fixtures** nach `docs/captures/` kopieren (nicht nur `live/`):
   - `idle-enq-nak.hex`
   - `shot-live-01.hex` (mindestens ein STX)
3. Sniffer- + Rust-Tests um die neuen Fixtures erweitern (wie `synthetic-shot.hex`)
4. Roadmap Phase-0-Zeilen auf DONE setzen

---

## Mindest-Capture-Satz (Definition of Done Phase 0 Hardware)

| Datei | Inhalt |
|---|---|
| `live/idle-enq-nak.hex` | nur ENQ/NAK |
| `live/shot-series.hex` | ≥3 echte STX-Frames |
| `live/burst-rapid.hex` | schnelle Folge |
| optional `live/getvars.hex` | Firmware-Info |

Synthetisch bleibt: `synthetic-shot.hex` (Regression, kein Live-Ersatz).
