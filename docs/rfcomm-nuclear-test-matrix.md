# RFCOMM Nuclear + Startup — Testmatrix

Stand: **Startup Nuclear** (Known BD_ADDR → 1× Forget→Pair→RFCOMM) + Badge/Setup Nuclear. Bond-Lookup = Diagnose, nicht Soft-A-Gate.

Plattform Lab-Runner: **Windows** (Classic SPP / PowerShell). macOS/Linux: RFCOMM-Produktpfad N/A; manuelle UI-Fälle nur wo relevant.

Hardware: KT RDT / Classic SPP, Ziel an und nah (außer Negativ-Labs).

---

## Gesamtrunner (Nuclear + Autoconnect)

Reihenfolge ist bewusst gestaffelt: wenig Stack-Last → Bond-Gate → Soft-Labs → Nuclear-Stress.

| Tier | IDs | Inhalt |
|------|-----|--------|
| A | N1, N6, N7 | Cold + Negativ |
| B | **N10** | Bond-Gate Soft↔Nuclear |
| C | N8, N9 | Soft einmal / Soft-oder-Nuclear |
| D | N2–N5 | Nuclear Core / Manager / Twice / Product |

```powershell
# Repo-Root — Full-Matrix (~15–25 min mit Hold):
powershell -ExecutionPolicy Bypass -File scripts\run-nuclear-matrix.ps1
# ohne 45s Product-Hold:
powershell -File scripts\run-nuclear-matrix.ps1 -SkipHold
# Teilmengen:
powershell -File scripts\run-nuclear-matrix.ps1 -Only cold,timeout,json
powershell -File scripts\run-nuclear-matrix.ps1 -Only bondgate
powershell -File scripts\run-nuclear-matrix.ps1 -Only auto,autonuc
powershell -File scripts\run-nuclear-matrix.ps1 -Only bondgate,auto,autonuc,reset
# aus src-tauri:
powershell -File ..\..\..\scripts\run-nuclear-matrix.ps1
```

`-Only`-Keys: `cold` `timeout` `json` `bondgate` `auto` `autonuc` `reset` `manager` `twice` `product`

---

## 1. Automatisierte Lab-Bins

| ID | Bin | Soll | HW |
|----|-----|------|-----|
| **N1** | `bt_cold_start` | Bond-Gate Soft → Linked **oder** Idle (kein Bond); kein Nuclear ohne User; kein Soft-Wake-Loop | optional |
| **N6** | `bt_nuclear_timeout` | Unerreichbare Addr → definierter Fehler, kein Hänger >120 s | optional |
| **N7** | `bt_json_corrupt` | Kaputte Known-JSON → kein Crash | optional |
| **N10** | `bt_bond_gate_matrix` | P1 Nuclear → P2 Soft(Bond) → P3 Forget → P4 Gate=Nuclear → P5 Nuclear | Pflicht |
| **N8** | `bt_auto_once` | Soft nur wenn Bond; sonst **PASS SKIP** | Pflicht |
| **N9** | `bt_auto_then_nuclear` | Soft wenn Bond, sonst Nuclear → Linked | Pflicht |
| **N2** | `bt_reset_connect` | Nuclear → Linked (~5–15 s) | Pflicht |
| **N3** | `bt_nuclear_smoke` | Manager Nuclear **ohne Soft** (`REDOT_SKIP_SOFT_AUTOCONNECT`) + Hold 20 s | Pflicht |
| **N4** | `bt_nuclear_twice` | Zwei Verbinden-Zyklen nacheinander | Pflicht |
| **N5** | `bt_product_smoke` | Cold Soft|Idle → ggf. Nuclear → Hold 45 s (`-SkipHold` überspringt) | Pflicht |

### Bond-Gate (N10) — Detail

Soll: **Bond OK → Soft**; **Bond weg → Soft überspringen → Nuclear**.

Soft+PIN-Hook kann ohne Bond trotzdem linken (Pair-on-Connect) — Gate = Bond-Check *vorher*.

| Phase | Inhalt | Soll |
|-------|--------|------|
| P1 | Nuclear establish, Socket drop | Bond+Link |
| P2 | Soft bei Bond | Linked ohne Forget |
| P3 | Nur OS-Bond entfernen | Bond weg, Known bleibt |
| P4 | Gate `bond_auth?` | Soft skip → Nuclear gewählt |
| P5 | Nuclear recovery | Linked wieder |

Exit N10: `11`=P1 … `15`=P5. Toast bei Soft (P2) möglich (AF_BTH).

**Lesart N8:** Known ohne Bond = SKIP; nach N10 (Bond da) sollte Soft PASS sein.
**Lesart N9:** Soft oder Nuclear → Linked muss PASS.

---

## 2. Manuelle UI — Kern (Abnahme)

| ID | Szenario | Schritte | Soll | Loop-Check | Ergebnis |
|----|----------|----------|------|------------|----------|
| **U1** | Erstes Einrichten | App ohne Known → Sheet | Suche → Nuclear → Linked | während Sheet kein Hintergrund-Soft | |
| **U1b** | Sheet abbrechen | „Später“ / Backdrop | Zurück; kein Leak; erneut öffnen ok | — | |
| **U2** | Alltag Verbinden | Known ohne Soft-Link, Badge Verbinden | Nuclear → Linked | **kein** Soft-Wake-Spam | |
| **U3** | JSON weg | Forget / JSON löschen | Einrichten / Sheet | — | |
| **U3b** | Bond da, JSON weg | Nur JSON löschen, OS-Bond bleibt | Start adoptiert Candidate → Soft oder Idle; kein Crash | kein Dauerloop | |
| **U4** | API ohne Target | `rfcommConnectReddot` | Fehler „einrichten“ | — | |
| **U5** | Link lost | Ziel aus bei Linked | Idle „tippe Verbinden“, **kein** Auto-Reconnect | Log: kein Soft-Wake-Loop | |
| **U6** | Zweites Verbinden | Nach U5 Badge | Nuclear wieder Linked | kein Soft-Wake-Loop | |
| **U7** | Session vs Link | Training start/stop bei Linked | Socket bleibt | — | |
| **U8** | App-Exit | Beenden, Bond bleibt | Kaltstart Soft wenn Bond OK | N1 | |
| **U9** | Andere S/N | Neues Gerät Setup-Scan | Neue BD_ADDR persistiert; Verbinden ok | — | |
| **U10** | Toast/PIN Erfolg | Nuclear beobachten | PIN `0000` auto → Linked | — | |
### UI — Fehler / Interaktion (Abnahme erweitert)

| ID | Szenario | Schritte | Soll | Ergebnis |
|----|----------|----------|------|----------|
| **U11** | Doppelklick Verbinden | 2× schnell Badge | Ein Nuclear; Badge „Verbinde…“; kein Doppel-Trigger | |
| **U12** | Training + Link lost | Serie läuft, Ziel aus | Sauberer Link-Verlust; Session fault/klar; **kein** hängender Socket; Verbinden danach ok | |
| **U13** | BT-Adapter aus | Adapter deaktivieren, Verbinden | Klare Fehlermeldung / NeedsPairing|Faulted, kein endloser Spinner ohne Reason | |
| **U14** | JSON korrupt | Known-JSON mutilieren, App-Start | Wie N7; UI Einrichten oder Bereit ohne Crash | |
| **U15** | PIN / Pair abgelehnt | Gerät aus / Pair-Modus aus bei Nuclear | Fehlerzustand + Reason; erneut Verbinden möglich | |
| **U16** | Mehrere RedDots in Reichweite | Zwei KT RDT sichtbar | Setup wählt einen Candidate (Name-Hint-Rank); keine stille Verwechslung ohne Scan | |

---

## 3. Soft-Wake-Regression (Anti-Ziel)

Nicht nur N1: bei **U2, U5, U6** und Idle nach Soft-Fail:

- Badge nicht dauernd „Verbinde…“ ohne Klick
- JSONL: keine `connect_fail`-Serien ohne User-Geste
- Owner nicht in Dauer-`reconnecting` / Soft-Wake-Loop

Bond-Gate Soft auf Start (einmalig, capped) ist **erlaubt**; Dauerloop ist **verboten**.

---

## 4. Diagnose / Logging (Abnahme)

| ID | Prüfung | Soll |
|----|---------|------|
| **D1** | Nuclear Erfolg | JSONL/`last_reason`: Forget/Pair/Link nachvollziehbar (`linked` / nuclear) |
| **D2** | Nuclear Fail (N6/U15) | Reason nicht leer; Status NeedsPairing|Faulted |

---

## 5. Abnahme-Checkliste

**Pflicht**

- [ ] N1, N6 Lab grün
- [ ] **N10** Bond-Gate (Soft bei Bond, Gate→Nuclear ohne Bond)
- [ ] N8–N9 Soft-Staffelung
- [ ] N2–N5 Nuclear-Stress
- [ ] U1, U1b, U2–U10 manuell ok
- [ ] U11–U15 manuell ok (Negativ/Interaktion)
- [ ] Soft-Wake-Regression (kein Dauerloop; U2/U5/U6)
- [ ] D1/D2 Logging stichprobenartig

**Optional / Nice-to-have**

- [ ] N7 JSON korrupt
- [ ] U16 Mehrfach-Geräte
- [ ] macOS/Linux: nur dokumentieren „N/A Classic RFCOMM“

**Explizit kein Widerspruch mehr:** U9 (Hardware-Vielfalt) und U10 (Auth-UX) gehören zur **Pflicht-Abnahme**, weil Nuclear ohne PIN-Erfolg und ohne Zweitgerät-Check produktseitig unvollständig wäre.

---

## 6. Bekannte Grenzen der Matrix

| Thema | Status |
|-------|--------|
| OS-Privacy-Dialog (BT) | Manuell U13-ähnlich; schwer automatisierbar |
| Parallele Invokes Server-seitig | U11 UI; Owner serialisiert Commands ohnehin |
| WinRT-Vergleich | `bt_winrt_simple` — nicht Produkt |
| Soft ohne Bond | Physisch möglich (Pair-on-Connect) — deshalb **Gate**, nicht Soft-Versuch |

Verwandt: Soft-Wake-Dauerloop und Soft-A-Start sind für den Produktpfad **überholt**; Produkt = Startup Nuclear (Known einmal) + Badge/Setup Nuclear.
Comparison-Labs `bt_softwake_ab` / `bt_master_auto`: nur mit `--features softwake-labs` (nicht Matrix-Pflicht, nicht Default-Build).

---

## 7. Startup-Nuclear Soak (Hardware-Abnahme)

**Arbeitsblatt:** [`baselines/rfcomm-soak/TEMPLATE-startup-nuclear.md`](./baselines/rfcomm-soak/TEMPLATE-startup-nuclear.md)
**JSONL:** `logs/rfcomm_connection.jsonl` — Zeilen `event=startup_nuclear` mit `runId` + `generation` (Toast-Notizen der Matrix zuordnen).

### Reihenfolge

1. **Smoke (2–3):** instrumentierte Zeile gegen Ablauf prüfen (`primaryOnly`, ein Nuclear, `retryScheduled=false`, Hook weg, linked plausibel) — erst dann zählen.
2. **20 Idle-Starts:** App voll beenden → Ziel Idle → Start; Toast/PIN aktiv ja/nein.
3. **10 Fehlstarts:** Ziel aus/OoR → Idle, kein Retry.
4. **10 Race/Exit:** Badge während Connecting, Setup open/close, Exit mid Forget/Pair/RFCOMM.
5. **10 Link-Lost:** Idle, nie Auto-Nuclear.
6. **Langlauf ≥4h:** Session start/stop; Socket+ENQ halten, kein neues Pair/Connect.

### Freigabe → erst dann Commit

20/20 Idle linked · 0 Toasts/PIN · 0 Doppel-Nuclear · 10/10 Fail Idle · 10/10 Race clean · Link-Lost ohne Auto-Nuclear · Langlauf stabil.
Bei Fail: **nicht** mitten im Soak patchen — Block zu Ende loggen, dann clustern.

### Entscheidungsregel nach Soak

1. **20/20 Idle-Starts linked, null Toasts:** Startup Nuclear produktionsreif; Soft-A/B und Start-Bond-Policy endgültig löschen.
2. **Link ok, aber Toast:** Nicht Soft-Wake zurück. Zuerst instrumentierte Schritte — welcher Aufruf die UI auslöst.
3. **Pair sporadisch fail:** Fail → Idle; keine zweite Auto-Nuclear. Danach Badge als expliziter Repair.
4. **RFCOMM fail nach Pair:** Nicht reflexartig Release/B. Fehlermuster aus Logs clustern.

