# Legacy contract inventory

**Stand:** Phase 2C Soft-API-/Command-Cleanup (nach Charakterisierung).  
**Produktvertrag:** Startup Nuclear; Soft-Wake Policy-API entfernt.

## Commands (`command.rs`) — nach Cleanup

| Symbol | Status |
|---|---|
| `Start`, `NuclearLink`, `SelectTarget`, `ForgetTarget`, Sink/Write, `PauseForSetup`, `CancelConnect`, `Shutdown` | **behalten** (Produkt) |
| `ConnectKnown`, `ReconnectNow`, `SystemResume`, `DeviceChanged` | **gelöscht** Phase 2C |

## Status (`status.rs`) — nach Cleanup

| Symbol | Status |
|---|---|
| `Idle`, `NeedsTarget`, `Discovering`, `Connecting`, `Linked`, `NeedsPairing`, `Faulted`, `ShuttingDown` | **behalten** |
| `Reconnecting` | **gelöscht** Phase 2C (Owner setzte nie; FE-Case defensiv) |

FE: `useLiveLinkStatus.mapRfcomm` mappt `"reconnecting"` noch nach `searching` —
Legacy-tolerant; Owner emittiert den String nicht mehr. Nach Kompatibilitätsfenster /
Event-Contract-Audit entfernen.

## Soft-Wake / Policy

| Symbol | Status |
|---|---|
| Soft-Wake Disposition/Gate-API | **gelöscht** |
| `BondLookup`, `ConnectOrigin`, `ConnectPhase`, `needs_pairing_ui` | **behalten** (Produkt/Diag) |
| `generation_tests.rs` | **gelöscht** |
| `bt_softwake_ab`, `bt_master_auto` | **Lab / Comparison-only** — nur mit `--features softwake-labs` |
| `bt_bonded_simple`, `bt_winrt_simple` | **Lab** (nicht Soft-Wake-Orchester; nicht hinter Gate) |
| `bt_bond_gate_matrix` | **Produktmatrix** (N10) — unangetastet |

## Doppel-Status

Unverändert: Domain `searching|connected|disconnected` vs Owner-Status.

## Charakterisierung

`product_owner_tests.rs` — Fake vs Real Tabelle siehe vorherige Phase; Commands ohne `ReconnectNow`/`SystemResume`.
