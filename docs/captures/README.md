# RedDot Captures

Hex-Captures des seriellen RedDot-Protokolls (9600 8N1). Dienen dem Sniffer-Replay und späteren Live-Vergleichen.

**Live-Hardware:** noch nicht angeschlossen. Workflow, Checkliste und Auto-Capture sind vorbereitet — siehe [CHECKLIST.md](./CHECKLIST.md) und [live/](./live/).

## Capture-Format (`.hex`)

- Encoding: UTF-8 Text
- Zeilen mit `#` sind Kommentare
- Leere Zeilen werden ignoriert
- Nutzdaten: whitespace-getrennte **Hex-Bytes** (`0`–`9`, `A`–`F`, je zwei Ziffern)
- Optionales Richtungspräfix am Zeilenanfang:
  - `RX:` / `<` — vom Ziel empfangen (wird beim Replay geparst)
  - `TX:` / `>` — vom Host gesendet (beim Replay ignoriert)
  - ohne Präfix — wie RX behandelt

Beispiel:

```text
# Idle keepalive
TX: 05
RX: 15

# Schuss-Frame (59 Bytes ab STX)
RX: 02 20 20 ...
TX: 06
```

Siehe auch `docs/protocol.md` (ENQ/NAK/STX 59-Byte-Schuss, ACK).

## Live-Capture-Workflow (wenn Hardware da ist)

Kurzfassung — Details und Checkboxen: **[CHECKLIST.md](./CHECKLIST.md)**.

1. Ziel verbinden, andere Software schließen, COM-Port notieren.
2. Ports listen:

   ```powershell
   cd "D:\Disag Reddot 2\tools\sniffer"
   npm run sniffer -- ports
   ```

3. Live-Sniff (ENQ-Poll ~300 ms, ACK nach Schuss). **Schreibt automatisch** nach `docs/captures/live/live-….hex`:

   ```powershell
   npm run sniffer -- port COM3 --duration 60000
   ```

   Oder fester Name:

   ```powershell
   npm run sniffer -- port COM3 --duration 60000 --out ../../docs/captures/live/shot-series.hex
   ```

4. Auswerten:

   ```powershell
   npm run sniffer -- replay ../../docs/captures/live/<datei>.hex
   npm run sniffer -- analyze ../../docs/captures/live/<datei>.hex
   ```

   `analyze` dump’t Header `[1..31]` und Trailer `[55..58]` byteweise (für `protocol.md`).

5. Sessions laut Checkliste: Idle → Schüsse → Burst → optional GetVars.
6. Offene Protokollfragen in `protocol.md` nachziehen; beste Captures als Golden Fixtures nach `docs/captures/` heben.

**Status:** Scaffolding ready — kein Gerät am Arbeitsplatz, daher noch keine echten Captures.

## Fixtures

| Datei | Inhalt |
|---|---|
| `synthetic-shot.hex` | Synthetisch: NAK, dann STX-Frame value=10.5, x=123, y=-45 |
| `live/*.template.hex` | Platzhalter für Idle / Schüsse / Burst / GetVars |
| `live/live-*.hex` | (kommt) Auto-Output vom Sniffer |

**Shared golden fixture:** `synthetic-shot.hex` is the single source of truth for protocol replay until live goldens exist. Both Rust (`apps/desktop/src-tauri/tests/arena_integrity.rs`) and the TypeScript sniffer (`tools/sniffer`) must load this same file — do not duplicate the hex elsewhere.
