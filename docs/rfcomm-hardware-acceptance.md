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

## Freigabekriterien

- [ ] Lab N1–N10 grün (oder dokumentierte SKIP)
- [ ] Soft nur bei Bond; Verbinden = Nuclear
- [ ] Kein Soft-Wake-Dauerloop nach Link lost
- [ ] Session-Stop beeinflusst Verbindung nicht
- [ ] Kein COM/PnP/Radio im Produktpfad
- [ ] Matrix oben bestanden
