# ADR 0014: Snapshot-Lifecycle und globale Retention

Status: angenommen, 2026-10-01. Befund F11, Ergänzung zu ADR 0010.

Ein Worker mit maximal 16 wartenden Aufträgen ersetzt Threads pro Snapshot.
Überlast und Fehler werden ausdrücklich protokolliert. Restore/Reset wartet
über die Generationsbarriere auf laufendes I/O und verwirft ältere Aufträge.
Beim endgültigen App-Exit werden Poll-Worker und Snapshot-Worker gejoint.
Beendete Poll-Handles werden auch im normalen Betrieb eingesammelt; ein
Worker darf seinen eigenen Handle nicht joinen. Abgebrochene Exit-Anfragen
lösen keinen irreversiblen Snapshot-Shutdown aus.

Retention gilt jetzt zusätzlich global: maximal 100 Session-Dateien, 30 Tage
und 512 MiB einschließlich latest.sqlite. Die neueste Session-Datei und
latest bleiben als Mindestbestand erhalten. Wenn dieser Mindestbestand
allein größer ist, wird die Budgetüberschreitung protokolliert. Bei jedem
erfolgreichen Snapshot wird bereinigt; externe Backups und Restore-Rollbacks
werden nicht automatisch gelöscht. I/O-Fehler bei der Bereinigung werden
gemeldet. Der Live-Datenbestand und seine vollständige Historie bleiben erhalten.

Tests prüfen Retention über verschiedene Sessions, minimale Rückfallkopien,
Windows-Dateisperren bei Veröffentlichung und Restore mit absichtlich
verzögertem altem Worker. Ein separater UI-Sicherungsstatus mit Wiederholung
und Langzeitmessung bleiben dokumentierte P2-Nacharbeiten; eprintln wird
damit noch nicht zu einem vollständigen rotierenden Betriebslog.
