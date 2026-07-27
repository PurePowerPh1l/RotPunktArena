# @rotpunktarena/sniffer

CLI zum Replay/Parsen/Analysieren von RedDot-Hex-Captures und optionalem Live-COM-Sniff (9600 8N1).

Nutzt `@rotpunktarena/protocol` (`packages/protocol`).

Live-Checkliste: `docs/captures/CHECKLIST.md`.

## Setup

Node.js 20+ (auf dieser Maschine per `winget install OpenJS.NodeJS.LTS`).

```powershell
cd "D:\RotPunktArena\tools\sniffer"
npm install
```

## Befehle

```powershell
# Fixture replayen
npm run sniffer -- replay ../../docs/captures/synthetic-shot.hex

# Header/Trailer dump (auch ohne Hardware an synthetischen Frames)
npm run sniffer -- analyze ../../docs/captures/synthetic-shot.hex

# Synthetischen Frame erzeugen
npm run sniffer -- synth

# COM-Ports listen (graceful ohne Hardware)
npm run sniffer -- ports

# Live-Sniff → schreibt automatisch docs/captures/live/live-….hex
npm run sniffer -- port COM3 --duration 60000

# Live-Sniff mit festem Dateinamen
npm run sniffer -- port COM3 --duration 60000 --out ../../docs/captures/live/shot-series.hex

# Nur Konsole, keine Datei
npm run sniffer -- port COM3 --duration 10000 --no-out

# Tests
npm test
```

Live-Validierung blockiert bis Hardware verfügbar — der Rest (Replay, Analyze, Templates) funktioniert offline.
