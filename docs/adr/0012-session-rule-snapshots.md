# ADR: Reproduzierbare Session-Regeln

Status: Accepted
Date: 2026-10-01

Migration 18 ergänzt `sessions.rules_json` mit Regelversion 1 und `rules_origin`.
Neue Sessions speichern im Start-Commit die Zehntelregel, effektive Serienlänge,
Wertungsart, Nachkauf-, Probe- und Teamregeln sowie Disziplin. Schuss-Ingest liest
die Zehntelregel ausschließlich aus dieser Session-Kopie. Arena-Ingest verwendet
nun explizit IMMEDIATE entsprechend dem Shot-Integrity-ADR.

Nach dem ersten Sessionstart sind ergebnisrelevante Wettkampfregeln unveränderlich.
Die Prüfung und Änderung erfolgen unter IMMEDIATE-Writer-Reservierung; gleichzeitige
Sessionstarts können sie nicht umgehen. Name/Datum bleiben editierbar. Änderungen
an Regeln erfordern einen neuen Wettkampf aus der vorhandenen Vorlage/Kopie.
Der separate Teamregel-Command prüft dieselbe Sperre. Ergebnisermittlung nutzt
weiterhin die dadurch eingefrorene Wettkampfkonfiguration; gespeicherte Scores
werden nicht neu berechnet.

Bestandsdaten: Es gibt keine gespeicherte vollständige Regelhistorie. Die Migration
kopiert den vorhandenen aktuellen Stand und kennzeichnet ihn ausdrücklich als
`legacy_current`; sie erfindet keine ursprünglichen Regeln und verändert keine
gespeicherten Schusswerte oder Rangfolgen. `captured` bedeutet atomar beim Start
erfasst. Bei einer früher bereits veränderten Legacy-Konfiguration bleibt deren
historische Ungewissheit bestehen; vor Release müssen betroffene Bestandsdaten
gesichtet werden. Laufende Legacy-Sessions verwenden nach Migration denselben
aktuellen Stand wie zuvor, danach bleibt er festgehalten.

Die Migration ist additiv. Ältere Apps ignorieren die neuen Felder und würden die
Regelsperre umgehen; Downgrade nur mit passendem vor dem Update erstelltem Backup
und ohne Übernahme neuer Wertungen. Die drei Regressionstests prüfen Bearbeitung
vor Start, Ablehnung während/nach Session, gespeicherte Zehntelregel trotz extern
geänderter aktueller Konfiguration und unveränderte Legacy-Schusswerte.
