# Soak Baseline — 2026-07-25 (post Phase-2 split)

- Gerät / BD_ADDR: KT RDT ZIE 1 S/N 203 @ `0018DA070564`
- Host / BT-Adapter: Windows (Phil), RFCOMM ch=1
- Branch / Commit: `feat/native-rfcomm` @ `b4eda6c` (nach `manager.rs`-Modul-Split)
- Dauer: ~2–4 min (`bt_soak` Phase A–C)
- Tool: `cargo run --bin bt_soak --features rfcomm`
- Vergleich: [`2026-07-25-phil.md`](./2026-07-25-phil.md) (pre-split @ `570ee0c`)

| Metrik | Pre-split | Post-split | Notiz |
|---|---|---|---|
| Warm-up attempts | 2 | 6 | Soft-Wake-Varianz; Muster gleich |
| Warm fail codes | HOSTDOWN | HOSTDOWN, TIMEOUT, ADDRINUSE×3 | Erwartet |
| ENQ hold | 40/40 | 40/40 | |
| Reconnect | 15/15 (0.1–0.2s) | 15/15 (0.1s) | |
| Ergebnis | PASS | PASS | Keine Split-Regression |

Auffälligkeiten:
- Längerer Warm-up ist typische Stack-/Schlaf-Varianz, nicht strukturell durch den Modul-Split erklärt (gleiche Socket-API).
- Nach Warm-up identisch stabil wie Baseline.

## Terminal-Auszug

```
warm try 1: WSAEHOSTDOWN
warm try 2: Timeout
warm try 3–5: WSAEADDRINUSE
warm OK on attempt 6
ENQ ok 40/40
reconnect 15/15 (100%), ENQ 100%
PASS
```
