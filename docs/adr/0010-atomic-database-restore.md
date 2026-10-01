# ADR: Atomarer Restore ohne Live-Dateiaustausch

Status: Accepted
Date: 2026-10-01

Restore/Reset ersetzen den Inhalt der weiterhin geöffneten Live-Datenbank mit der
SQLite-Backup-API. Die bisherige Live-Datei wird niemals entfernt oder durch eine
In-Memory-Verbindung ersetzt. SQLite hält die Zieländerung in einer Transaktion;
unvollständige Kopien rollen beim Beenden der Backup-Operation zurück.
Siehe [SQLite Backup API](https://www.sqlite.org/c3ref/backup_finish.html).

Vorbereitung: Backup read-only öffnen, Integrität/Fremdschlüssel und bekannte,
lückenlose Migrationsfolge prüfen; konsistente Kopie in eine eigene Staging-Datei
erstellen; dort migrieren und Kernschema prüfen. Neuere/unbekannte Schemas,
Fremddatenbanken und defekte Dateien werden vor der Live-Mutation abgewiesen.
Reset bereitet auf demselben Weg eine frisch migrierte leere DB vor.

Wartung serialisiert Start/Resume, stoppt Poll-Worker und wartet auf alle alten
Worker-Handles. Anschließend sperrt sie Ingest und Engine-DB. Die Snapshot-Barriere
wartet auf laufende Snapshot-I/O samt Connection-Close; eine neue Generation
verwirft alte Queue-Jobs. Der Snapshot-Worker hat eine Queue mit maximal 16 Jobs.
Die vollständige Retention-/App-Shutdown-Erweiterung ist das separate Finding F11.

Vor der Übernahme entsteht `restore-rollback-<UUID>.sqlite` neben der Live-DB,
validiert und unverändert aufbewahrt. Eine fehlgeschlagene Ersatzprüfung kopiert
diesen Rückfallstand zurück. Fehler enthalten bei erfolglosem Rollback dessen
Pfad. UI-Daten werden ausschließlich nach erfolgreicher Übernahme geleert.
Die Live-Verbindung bleibt in jedem Fehlerpfad dateibasiert. Externe SQLite-
Writer verursachen einen sichtbaren Kopierfehler statt unbegrenzter Wiederholung.

Wiederanlauf: SQLite-WAL-Recovery entscheidet nach Prozess-/OS-Ausfall über den
atomaren Ziel-Commit. Bei erfolgreichem Restore bleibt der Rückfallstand für einen
bewussten erneuten Restore erhalten. Rückfallstände nicht automatisch löschen;
sie enthalten personenbezogene Daten und müssen in die Betriebs-Retention fallen.
Kein Schemawechsel, Backup-Quelle bleibt unverändert. Unbekannte neuere Backups
benötigen eine passende App-Version.

Regressionen: ungültige/missing/neuere Backups, nach einer Seite abgebrochene
Kopie mit Reopen, Fehler bei Ersatzprüfung mit Rollback/Retry, fehlgeschlagene
Rückfall-Dateierstellung und erfolgreicher Reset. Windows-Dateisperren,
Datenträger-voll und Strom-/OS-Ausfall bleiben manuell abzunehmen.

## Ergänzung: Vollständiger Serienreset

Der Trainingsreset besitzt lifecycle_gate und ingest_gate vom Lesen der alten
Session über Abschluss und Projektion bis zum Start der neuen Session. Interne
Start-/End-Helfer übernehmen bereits gehaltene Locks. Restore/Reset der gesamten
DB kann nicht in die Lücke zwischen Serienende und Neustart treten. Ein Test
verzögert den neuen Start, bestätigt die blockierte Wartung und prüft anschließend
die Ablehnung wegen der neu laufenden Session sowie den erhaltenen Datenbestand.
Gezieltes Recovery-Schließen und Training-Speichern halten ebenfalls ingest_gate
von der Session-Auswahl bis zum Abschluss. Ein Test prüft den Erhalt einer
anderen aktuellen Session und den idempotenten Abschluss ohne rekursiven Lock.
