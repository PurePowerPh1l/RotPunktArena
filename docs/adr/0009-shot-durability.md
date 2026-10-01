# ADR: FULL-Durabilität für bestätigte Schüsse

Status: Accepted
Date: 2026-10-01

Alle Produktverbindungen verwenden WAL mit `synchronous=FULL`, Foreign Keys und
3 s Busy-Timeout. Erfolgreicher Arena-Commit ist weiterhin Voraussetzung für ACK
und Anzeige. SQLite synchronisiert unter FULL das WAL bei jedem Commit; NORMAL
kann bei OS-Absturz oder Stromausfall zuletzt committete Transaktionen verlieren.
Siehe [SQLite synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous)
und [WAL](https://www.sqlite.org/wal.html).

Die Zusage gilt bei korrekter Umsetzung der Sync-Aufrufe durch OS, Dateisystem
und Datenträger. Hardwaredefekte, fehlerhafte Storage-Firmware oder verlorene
Datenträger erfordern weiterhin Backups. Ein Prozess-Crash-/Reopen-Test beweist
keinen Stromausfallschutz. Strom-/OS-Ausfall und reales Geräte-ACK/Retry müssen
auf dem Wettkampf-PC vor Release manuell abgenommen werden.

Messung am 2026-10-01 auf diesem Windows-Entwicklungsrechner, Debug-Build,
isolierte temporäre DB, 200 echte Arena-Ingest-Commits ohne Snapshot-Worker:

| Modus | Median | p95 | Maximum |
|---|---:|---:|---:|
| NORMAL | 0,189 ms | 0,381 ms | 4,915 ms |
| FULL | 1,357 ms | 2,535 ms | 4,469 ms |

Das ist eine lokale Beobachtung, kein Leistungs-Gate für andere Datenträger.
`db::durability_tests` prüft FULL bei jeder Wiederöffnung, misst beide Modi und
prüft alle 200 gespeicherten Schüsse nach Reopen. NORMAL wird nur lokal auf der
Testverbindung gesetzt. Kein Schemawechsel; Rollback auf ältere App-Versionen
schwächt die Durabilität wieder auf deren NORMAL-Konfiguration ab.
