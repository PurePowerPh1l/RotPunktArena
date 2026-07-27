# Live Captures

Echte COM-Mitschnitte landen hier (automatisch vom Sniffer oder manuell).  
**Noch keine Hardware-Captures** — Ordner und Templates sind vorbereitet.

## Automatisch speichern

```powershell
cd "D:\Disag Reddot 2\tools\sniffer"
npm run sniffer -- port COM3 --duration 60000
# → schreibt docs/captures/live/live-YYYYMMDD-HHMMSS.hex
```

Expliziter Pfad:

```powershell
npm run sniffer -- port COM3 --duration 60000 --out ../../docs/captures/live/shot-series.hex
```

## Nach dem Capture

```powershell
npm run sniffer -- replay ../../docs/captures/live/<datei>.hex
npm run sniffer -- analyze ../../docs/captures/live/<datei>.hex
```

Checkliste: [../CHECKLIST.md](../CHECKLIST.md)

## Geplante Dateinamen

| Datei | Zweck |
|---|---|
| `idle-enq-nak.hex` | Keepalive ENQ/NAK |
| `shot-series.hex` | mehrere echte Schüsse |
| `burst-rapid.hex` | schnelle Schussfolge |
| `getvars.hex` | DC1 GetVars |

Templates (nur Kommentare, zum Ausfüllen): `*.template.hex` in diesem Ordner.
