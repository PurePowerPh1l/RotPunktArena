# RotPunktArena (Phase 1 MVP)

Lokaler Tauri-Client: Live-Stand mit Simulator, SQLite-Event-Log, Transport-Adapter.

## Voraussetzungen

- Node.js 20+
- Rust (stable) + MSVC Build Tools (Windows)

## Start

```bash
cd "D:\RotPunktArena"
npm install
npm run desktop:dev
```

Nur Frontend (ohne Tauri-Shell):

```bash
npm run dev --workspace=@rotpunktarena/desktop
```

## Serial (optional)

Standard-Build nutzt den **Simulator**. Echte COM/BT-SPP-Unterstützung:

```bash
cd apps/desktop/src-tauri
cargo build --features serial
```

Siehe [docs/transport.md](../../docs/transport.md) und [docs/protocol.md](../../docs/protocol.md).
