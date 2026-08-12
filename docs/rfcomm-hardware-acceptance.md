# Hardware-Abnahmematrix — Native RFCOMM (Bond-Gate)

Freigabe nur wenn Zeilen „OK“ oder bewusst „N/A“. Primäre Lab-Matrix: [`rfcomm-nuclear-test-matrix.md`](./rfcomm-nuclear-test-matrix.md).

| # | Szenario | Soll | Ergebnis | Datum / Tester |
|---|---|---|---|---|
| 1 | App-Start mit Known + Bond | Soft → Linked (kein Nuclear ohne User) | | |
| 2 | App-Start Known ohne Bond | Idle „Verbinden“; Badge = Nuclear | | |
| 3 | Kein Known | Setup-Sheet; Scan → Nuclear → Linked | | |
| 4 | Badge Verbinden | Nuclear → Linked | | |
| 5 | 100 Session start/stop | Socket bleibt; ENQ weiter; Schüsse OK | | |
| 6 | Link lost (Ziel aus) | Idle „tippe Verbinden“; **kein** Soft-Wake-Loop | | |
| 7 | Zweites Verbinden nach Lost | Nuclear wieder Linked | | |
| 8 | Forget Target | NeedsTarget; Sheet | | |
| 9 | Normal Exit / Restart | Bond bleibt; Soft beim nächsten Start wenn Bond OK | | |
| 10 | Soak ≥ 2 h Linked | Kein Handle-Leck; stabile Schüsse | | |
| 11 | UI-Begriffe | Kein COM / Repair / Port im Normalflow | | |
| 12 | Gerätewechsel A→B | Setup-Liste zeigt beide; Tap auf B → Nuclear auf B; A-Bond weg; `rfcomm_devices.json` active=B | **offen — braucht Zweitgerät** | |
| 13 | Gerätewechsel B→A (zurück) | Wie 12 in Gegenrichtung; kein manuelles „Gerät vergessen“ nötig | **offen — braucht Zweitgerät** | |
| 14 | Zwei RedDots gleichzeitig sichtbar | Scan listet beide (paired + nearby, dedupliziert); kein Auto-Connect ohne Tap | **offen — braucht Zweitgerät** | |
| 15 | Wechsel während Linked | Settings „Anderes Gerät verbinden“ → Sheet öffnet; alter Link fällt erst beim Scan (PauseForSetup) | **offen — braucht Zweitgerät** | |
| 16 | Legacy-Migration | Update mit vorhandener `rfcomm_known_target.json` → Gerät bleibt Known, Datei migriert zu `rfcomm_devices.json` | | |
| 17 | Gerätegedächtnis Ein-Tap | Settings: gemerktes B tippen → Nuclear Switch ohne Nearby-Scan nötig (Gerät muss erreichbar sein) | **offen — braucht Zweitgerät** | |
| 18 | Gerätegedächtnis Vergessen | Nicht-aktives Gerät „Vergessen“ entfernt nur JSON-Eintrag; aktives → Bond+Link weg | | |

## Freigabekriterien

- [ ] Lab N1–N10 grün (oder dokumentierte SKIP)
- [ ] Soft nur bei Bond; Verbinden = Nuclear
- [ ] Kein Soft-Wake-Dauerloop nach Link lost
- [ ] Session-Stop beeinflusst Verbindung nicht
- [ ] Kein COM/PnP/Radio im Produktpfad
- [ ] Matrix oben bestanden
