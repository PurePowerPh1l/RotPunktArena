# Codebase Cleanup Inventory

**Stand:** Soft-Wake Lab-Bins hinter `softwake-labs` (`chore/code-hygiene`)
**Regel:** Nur beschreiben — keine Umbauten in diesem Dokument.
Ein Modul ist ein Kandidat, wenn es **mehrere unabhängige Änderungsgründe** hat, nicht weil es „lang“ ist.

**Produktvertrag:** Soft-Wake Policy-API entfernt. Start = Startup Nuclear (PrimaryOnly); Verbinden = NuclearLink; Link-Lost → Idle.
**Charakterisierung:** `connection/product_owner_tests.rs`.
**Lab ≠ Produkt:** Soft-Wake Comparison-Bins nur mit `--features softwake-labs` (siehe `soft-wake-caller-matrix.md`).
**Nächster Schritt:** LiveStandView über Hooks; danach `discovery` nur bei echten Grenzen; Owner-Extrakt erst mit Messsignal.

Priorität der nächsten Reviews (max. 5): siehe Abschnitt [Top-5](#top-5-kandidaten).

---

## Inventar

| Modul | ~Zeilen | Verantwortung heute | Auffälligkeit | Risiko | Entscheidung |
|---|---:|---|---|---|---|
| `connection/owner.rs` | ~573 | Single-flight Link-Lifecycle: Commands, Startup Nuclear, Badge Nuclear, Link-Lost → Idle | Command-Dispatch + Nuclear-Wait-Loop + Linked-Pump in einer Datei | Hoch | **Review** zuerst (Charakterisierungstests) |
| `connection/nuclear.rs` | ~283 | Forget → Pair → RFCOMM als einmalige Operation | Kernvertrag; Lab + Produkt | Hoch | **Keep** — nicht ohne Matrix ändern |
| `connection/setup_flow.rs` | ~262 | Scan + First Setup + Attach an in-flight Nuclear | UI-/BT-Grenze | Mittel | **Review** Contract |
| `connection/keepalive.rs` | ~88 | ENQ / Read / Link-Lost / Sink-Fanout | schmal, klar | Niedrig | **Keep** |
| `connection/connect_policy.rs` | ~120 | BondLookup, Origin, Phase | Soft-Wake-API entfernt (Phase 2C) | Niedrig | **Keep** |
| `connection/manager.rs` | ~61 | Facade / re-exports nach Split | dünn | Niedrig | **Keep** |
| `connection/command.rs` | ~35 | Command-Enum | Tote Varianten entfernt | Niedrig | **Keep** |
| `transport/rfcomm/discovery.rs` | ~846 | Enumerate, Bond, Pair, Forget, PairApiReport | Discovery + Pairing + Diagnose nebeneinander | Hoch | **Extract** Kandidat (stärkster Split) |
| `transport/rfcomm/auth_hook.rs` | ~500 | PIN-Hook install/uninstall | Windows-FFI + Produktpfad | Hoch | **Keep** / Review nur bei Hook-Bugs |
| `commands/live.rs` | ~216 | Tauri-DTO + Commands | Contract Drift möglich | Mittel | **Review** DTOs |
| `protocol.rs` | ~322 | Bytes → Frames/Events | Datenintegrität | Hoch | **Keep** — Golden Tests prüfen, nicht „verschönern“ |
| `connection/bridge.rs` | ~81 | Session-Sink an/ab, Bytes weitergeben | Link/Session-Grenze | Hoch | **Review** Ownership |
| `engine/poll/connect.rs` | (engine) | Session-Transport open/close | darf Socket nicht schließen | Hoch | **Review** Ownership |
| `LiveStandView.tsx` | ~782 | Screen-Komposition, Sheets, Session-UI | UI-God-Component-Kandidat | Mittel | **Extract** Komponentenkarte |
| `useLiveLinkStatus.ts` | ~91 | Polling + UI-Projektion | Statusmapping Rust↔TS | Mittel | **Review** Contract-Test |
| `LiveLinkBadge.tsx` | ~239 | Badge CTA / Cancel / Diag | Repair-Policy sichtbar | Mittel | **Keep** |
| `RedDotWakeSheet.tsx` | ~107 | Wake-Guidance, Nuclear-CTA | neu, schmal | Niedrig | **Keep** |
| `RedDotSetupSheet.tsx` | ~186 | First Setup UI | Setup-Contract | Mittel | **Keep** |

---

## Top-5 Kandidaten

| # | Modul | Entscheidung | Nächster Schritt (kein Code hier) |
|---|---|---|---|
| 1 | `owner.rs` | Review | Charakterisierungstests: Start, NuclearLink, Pause, Shutdown, Link-Lost, Badge während Connecting, stale Generation |
| 2 | `discovery.rs` | Extract Kandidat | Inventar der Symbole: Enum vs Bond vs Pair vs Report — Split nur wenn verbotene Abhängigkeit verschwindet |
| 3 | Soft-Wake Lab-Bins | Keep (gated) | `bt_softwake_ab` / `bt_master_auto` hinter `softwake-labs`; Extra-Crate optional später |
| 4 | `LiveStandView.tsx` | Extract | Sheet-Steuerung vs Session-UI vs Status trennen (nach Owner-Tests) |
| 5 | `commands/live.rs` | Review | DTO-Parität Domain/TS; keine Produktlogik in Commands |

---

## God-File-Audit (Kurz)

Kriterien (≥2 → Kandidat): mehrere Domänen · mehrere Ressourcenbesitzer · große State-`match`-Blöcke · unverbundene Abhängigkeiten · ändert sich bei vielen Features · nicht in einem Satz erklärbar · Tests brauchen riesige Setups.

| Modul | Kriterien getroffen | Bemerkung |
|---|---|---|
| `owner.rs` | Domänen + State-match + Feature-Hotspot | Nach Startup-Nuclear klarer, aber Dispatch+Worker-Wait noch vermischt |
| `discovery.rs` | Domänen + FFI + Diagnose | Stärkster Split-Kandidat |
| `LiveStandView.tsx` | Komposition + Sheets + Session | UI-Extraktion nach Stabilitätsfenster |
| `protocol.rs` | — | Lang genug, aber eine Domäne → **kein** God-File-Refactor |
| `nuclear.rs` | — | Eine Operation → Keep |

---

## Modulgrenzen (Soll — Phase 4)

| Modul | Darf | Darf nicht |
|---|---|---|
| `owner.rs` | Commands serialisieren, Generation, State-Transition | FFI, Pairing-Details, JSON parsen, UI-Texte erfinden |
| `nuclear.rs` | Forget→Pair→RFCOMM einmal | Commands pollen, UI-Status frei, andere Targets wählen |
| `keepalive.rs` | ENQ, Read, Link-Lost, Sink | Pairing, Persistenz, Connect-Policy |
| `discovery.rs` | Enumerieren, Bond lesen, pair/forget | Produktzustand, Tauri-DTO, UI-Auswahl |
| `persist.rs` | KnownTarget laden/speichern/löschen | Bluetooth-APIs, Connects, Status |
| `commands/live.rs` | DTO validieren, Owner-Command senden | eigene Connection-Logik |
| `bridge.rs` | Sink an/ab, Session-Bytes | Socket öffnen, Pairing, Retry |
| `protocol.rs` | Bytes → Frames/Events | RFCOMM, SQLite, React, Tauri |
| React Hook | Polling, Projektion, Cleanup | Pairing-Policy, Produktentscheidungen |

---

## Cleanup-Backlog (Gates)

```text
Inventar → Befund → Beweis → kleiner Refactor → Tests → Review-Stopp
```

1. ~~Inventar~~ (dieses Dokument)
2. ~~Phase-1 Vertragssprache + Soft-Wake-Matrix~~
3. ~~Charakterisierungstests Owner~~ — `product_owner_tests.rs`
4. ~~Soft-API + Legacy-Tests entfernt~~ (Phase 2C)
5. ~~Soft-Wake Lab-Bins feature-gegatet~~ (`softwake-labs`)
6. Erst dann optionale Extrakte (`LiveStand` Hooks, dann `discovery`)

**Kein weiterer RFCOMM-Mechanik-Umbau ohne neues Messsignal** (Hardware-Baseline `5b1dcd3` / Merge `47524dd`).
