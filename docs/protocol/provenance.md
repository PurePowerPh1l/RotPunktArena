# Protocol provenance

Kurzstatus zur Herkunft der Protokollbeschreibung. Keine Behauptungen über Inhalte außerhalb dieses Repos.

## Im Repo nachweisbar

| Artefakt | Rolle |
|---|---|
| [`docs/protocol.md`](../protocol.md) | Produkt-Spezifikation (Geräteverhalten) |
| [`packages/protocol/`](../../packages/protocol/) | TypeScript-Parser + Tests |
| [`apps/desktop/src-tauri/src/protocol.rs`](../../apps/desktop/src-tauri/src/protocol.rs) | Rust-Parser |
| [`docs/captures/`](../captures/) | Hex-Fixtures / Capture-Workflow |
| [`tools/sniffer/`](../../tools/sniffer/) | Capture / Replay / Analyze |

## Nicht im Repo versioniert

Lokale Vendor-/Extract-/Tool-Ordner und Root-PDFs sind bewusst **nicht** Teil der Produktquelle (siehe `.gitignore`). Inhalt und Vollständigkeit solcher lokalen Artefakte: **nicht nachweisbar** aus dem Git-Stand allein.

## Validierung

| Thema | Status |
|---|---|
| Frame-Offsets Wert / Distanz / X / Y | in Parsern implementiert; Live-Hardware **pending validation** |
| Header `[1..31]` / Trailer `[55..58]` | **pending validation** |
| ENQ/NAK/STX/ACK-Hot-Path | in Code + Simulator; Live **pending validation** |
| GetVars / Disctype / Reset-Folgen | Kommandos dokumentiert; Live-Antworten **pending validation** |
| Reconnect-Timing | Softwareskizze in `protocol.md`; Live **pending validation** |

Golden-Fixtures aus echten Geräten: noch nicht vorhanden (Stand Captures: synthetisch / Templates).
