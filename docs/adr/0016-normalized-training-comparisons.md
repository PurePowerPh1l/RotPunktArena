# ADR 0016: Vergleichsbasis für Trainingsmetriken

Status: angenommen, 2026-10-01. Befund F13, begrenzter Teil umgesetzt.

Liga, Transfer Training/Wettkampf, Punkte-Trend, Chart und vergleichende
Serienkennzahlen verwenden `punkteTotal / shotCount * 10`. Reale Schusszahl,
persistierte Serienwerte, Wettkampfergebnisse und volumenabhängige XP bleiben
unverändert. Der Gesamt-Teilerdurchschnitt im Fenster ist nach Schusszahl
gewichtet. UI-Labels nennen die Zehnerskala. Tests belegen gleiche Liga und
Transfer bei gleicher Leistung für 5/10/20/30 Schüsse und gemischte Längen.

Arena und Historie laden höchstens 200 Serien. Die Historie nennt Zeitraum,
Fenstergrenze und fehlende Lifetime-Gesamtstatistik ausdrücklich. Die Liga
je Schütze basiert auf dem globalen zuletzt geladenen Fenster; seltene Schützen
können darin fehlen. Auch dies wird angezeigt. Es wird keine vollständige
Historienberechnung vorgetäuscht.

Ein eigener Follow-up-PR muss serverseitige Lifetime-Aggregate, paginierte
Historie und dauerhaft reproduzierbare Liga über die vollständige Historie
bereitstellen, inklusive mehr als 200 Serien und seltener Schützen. Ebenfalls
zu prüfen sind längenabhängige persönliche Ziele und Rivalen-/Pulse-Anzeigen.
Diese UI-Gamification ist kein Bestandteil der gespeicherten Wertungen.
