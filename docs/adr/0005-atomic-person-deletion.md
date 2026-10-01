# ADR: Atomare Personenlöschung

Status: Accepted
Date: 2026-10-01

Personenlöschung reserviert mit `BEGIN IMMEDIATE` zuerst den SQLite-Writer.
Die Prüfung offener Sessions (direkte Personenreferenz oder Starterreferenz),
Entkopplung abgeschlossener Wettkampfserien, Löschung der Starter und aller
Trainingsdaten sowie der Person erfolgen auf derselben Verbindung in einer
Transaktion. Jeder Fehler rollt die gesamte Änderung zurück. Offene Sessions
müssen vor der Löschung beendet werden; unterbrochene Sessions zählen ebenfalls
als offen. Abgeschlossene Wettkampfserien bleiben gemäß bisheriger Semantik
erhalten; ihre Starter-/Personenreferenzen werden entfernt.

Die Writer-Reservierung verhindert, dass eine andere Verbindung zwischen
Referenzprüfung und Löschung eine Session für die Person startet. Die vorhandenen
Fremdschlüssel verhindern neue Referenzen auf eine anschließend gelöschte Person.
Es gibt keine Schemaänderung.

Regression: `db::people::tests` injiziert Fehler an allen acht Mutationsschritten,
prüft vollständigen Rollback und Wiederholung sowie beide offenen Referenzpfade.
