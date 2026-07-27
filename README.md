# RedDot Arena

Lokaler Stand-Client als Ersatz für RedDotView (Phase 1 MVP).

## Struktur

| Pfad | Inhalt |
|---|---|
| `apps/desktop` | Tauri 2 + React/Vite Live-Stand |
| `packages/protocol` | TypeScript-Parser (Serielles RedDot-Protokoll) |
| `packages/domain` | Gemeinsame Domain-Typen |
| `docs/reddot-arena-review.md` | Architektur- & Feature-Review |
| `docs/protocol.md` | Serielles Protokoll |
| `docs/transport.md` | Transport-Adapter (COM / Simulator / Phase T TCP) |
| `docs/README.md` | Docs-Index |

## Start

Voraussetzung: Node.js LTS, Rust stable, Windows MSVC Build Tools.

```bash
npm install
npm run desktop:dev
```

Ohne Hardware: **Simulator starten** → Verbindungsstatus → Schüsse senden / Auto-Schuss.

## Build

```bash
npm run desktop:build
```

Installer liegen unter `apps/desktop/src-tauri/target/release/bundle/`.
