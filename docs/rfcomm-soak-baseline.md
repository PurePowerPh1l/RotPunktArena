# RFCOMM Soak-Baseline (Phase 1)

Vor jeder strukturellen Änderung am Connection-Stack (`manager.rs`-Split, Connect-Konsolidierung, TCP-Vorbereitung) eine **Baseline** mit realem Zielgerät archivieren und danach erneut vergleichen.

## Lauf

Voraussetzungen: Windows, gekoppeltes RedDot (authenticated Bond), Ziel an und nah.

```bash
cd apps/desktop/src-tauri
cargo run --bin bt_soak --features rfcomm
```

Für eine 4h-Baseline den Soak-Lauf mehrfach wiederholen oder `bt_soak` Phase B/C verlängern und parallel `logs/rfcomm_connection.jsonl` mitschneiden (App oder Soak).

Empfohlen parallel zur App:

1. Known-Target linked lassen (≥ 4 h).
2. JSONL nicht löschen.
3. Am Ende Metriken ausfüllen (Vorlage unten).

## Archiv

Ablage: `docs/baselines/rfcomm-soak/` (gitignored Rohlogs optional; **diese Markdown-Tabelle committen**).

Dateiname: `YYYY-MM-DD-hostname.md` (Kopie der Vorlage).

## Vorlage (ausfüllen)

```markdown
# Soak Baseline — YYYY-MM-DD

- Gerät / BD_ADDR:
- Host / BT-Adapter:
- Branch / Commit:
- Dauer:
- Tool: bt_soak / App linked

| Metrik | Wert | Notiz |
|---|---|---|
| Warm-up attempts bis first linked | | |
| Reconnect-Zyklen (absichtlich / unfreiwillig) | | |
| connect_fail total (JSONL) | | |
| Soft-Wake (HOSTDOWN/TIMEOUT/ADDRINUSE) | | |
| AuthStop (WSAEACCES) | | |
| enq_fail / read_fail | | |
| IO_FAIL_LIMIT-Ereignisse (Link lost) | | |
| Längste Reconnecting-Phase | | |
| Linked-Anteil (geschätzt) | | |

Auffälligkeiten:
-
```

## Abnahme Phase 1

- [ ] Mindestens eine ausgefüllte Baseline-Datei unter `docs/baselines/rfcomm-soak/`
- [ ] Referenzwerte für Soft-Wake-Rate und Reconnect-Frequenz notiert
- [ ] Nach Refactor: zweite Messung, Abweichungen begründet
