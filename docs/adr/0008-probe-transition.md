# ADR: Probewechsel und Ingest koordinieren

Status: Accepted
Date: 2026-10-01

Ein Engine-Gate umfasst Ingest vom Vorabcheck über SQLite-Commit bis zur
Live-Projektion und Schussbenachrichtigung. Der Probewechsel hält dasselbe Gate
bis nach Phasenänderung, Event-Commit, Live-Reset und Probe-Benachrichtigung.
Damit kann kein erster Wertungsschuss zwischen Commit und Reset verschwinden.
Phase und `probe_finished` werden gemeinsam in einer IMMEDIATE-Transaktion
gespeichert. Fehler lassen die Probephase vollständig bestehen.

Lock-Reihenfolge: Gate → DB → Inner (DB-/Inner-Guards bleiben kurz; kein Inner-
Guard wird während eines DB-Aufrufs gehalten). Der automatische Abschluss wird
erst nach Freigabe des Ingest-Gates aufgerufen.

Regressionen: Event-Insert-Fehler mit Rollback/Retry, Klassifikation und Index
des ersten Wertungsschusses sowie ein verzögerter Probe-Benachrichtigungsweg mit
konkurrierendem Ingest. Die vollständige Live-Event-Versionierung folgt separat.
