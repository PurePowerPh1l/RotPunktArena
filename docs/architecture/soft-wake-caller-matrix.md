# Soft-Wake caller matrix

**Stand:** Soft-Wake Lab-Bins hinter Feature `softwake-labs` (nicht in `default`).
**Produkt:** Soft-Wake retired; Startup Nuclear abgesichert in `product_owner_tests.rs`.

## Entfernt (Phase 2C)

| Symbol / Artefakt | Früher | Jetzt |
|---|---|---|
| `SOFT_WAKE_*`, `disposition_after_fail`, `classify_connect_fail`, `may_start_rfcomm`, `allows_release_retry`, `ConnectFailKind`, … | Unit-Test only | **gelöscht** aus `connect_policy.rs` |
| `generation_tests.rs` | Legacy Soft-Wake FakeOwner | **gelöscht** |
| `ConnectionStatus::Reconnecting` | nie von Produkt-Owner gesetzt | **gelöscht** (FE mappt `"reconnecting"` defensiv weiter) |
| `ConnectKnown`, `ReconnectNow`, `SystemResume`, `DeviceChanged` | tot / no-op / Alias | **gelöscht** |

## Lab-Bins — Klassifikation

| Bin | Klasse | Feature-Gate | Hinweis |
|---|---|---|---|
| `bt_softwake_ab` | **Comparison-only** | `softwake-labs` (+ `rfcomm`) | Soft-Wake Pause A/B; kein Produktpfad |
| `bt_master_auto` | **Comparison-only** | `softwake-labs` (+ `rfcomm`) | Master-style Soft-Wake-Schleife; kein Nuclear |
| `bt_bonded_simple` | **Lab** | nur `rfcomm` | Minimal gekoppelt→connect; **kein** Soft-Wake-Orchester (nur markiert) |
| `bt_winrt_simple` | **Lab / Comparison** | nur `rfcomm` | WinRT vs Winsock; Matrix: nicht Produkt (nur markiert) |
| `bt_bond_gate_matrix` | **Produktmatrix-Lab** (N10) | nur `rfcomm` | Matrix-Bin — **nicht** hinter `softwake-labs` |

### Soft-Wake Labs starten

```text
cargo run -p reddot-desktop --bin bt_softwake_ab --features softwake-labs
cargo run -p reddot-desktop --bin bt_master_auto --features softwake-labs
```

Default-Features enthalten `rfcomm`, aber **nicht** `softwake-labs`. Ohne Gate erscheinen diese Bins nicht als normaler Produkt-Einstieg.

## Sonst noch Lab (nicht Soft-Wake-Policy)

| Artefakt | Urteil |
|---|---|
| `connection/backoff.rs` (`#[cfg(test)]`) | reine Unit-Tests, kein Produktpfad |
| `ConnectPhase::Backoff` Wire-String | behalten (Owner setzt ihn nicht) |

## Produkt-Absicherung

`connection/product_owner_tests.rs` — Startup Nuclear Charakterisierung.
