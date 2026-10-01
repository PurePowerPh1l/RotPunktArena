# ADR: Persistenz vor Session-Abschluss

Status: Accepted
Date: 2026-10-01

Recovery am Schusslimit und automatischer Serienabschluss verwenden denselben
Abschlusspfad. `series_complete` wird erst nach erfolgreichem `finish_live_session`
gesetzt. Der Inner-Guard endet vor dem Snapshot; ein nicht-rekursiver Mutex wird
nicht erneut gesperrt. Probe, unbegrenzte und unvollständige Serien schließen nicht
automatisch ab. Wiederholter erfolgreicher Abschluss erzeugt kein weiteres Event.

Bei Persistenzfehlern bleiben DB und Live-Session offen und `series_complete` falsch.
Der Worker wird angehalten; der Bediener kann den Abschluss erneut ausführen.
IPC-Aufrufer erhalten den Fehler. Der Poll-Pfad veröffentlicht einen sichtbaren
Verbindungsfehler direkt, weil der Stop die Worker-Generation bereits geändert hat.
Ein erfolgloser Abschluss erzeugt kein `series_complete`-Event.

Regressionen unter `engine::series::tests` prüfen die Recovery-Guard-Grenze mit
Timeout, Rollback per Event-Trigger, erneuten Abschluss und die Ausschlussregeln.
Die Hardware-Anzeige und der echte Recovery-Gate bleiben Teil der manuellen Abnahme.

## Ergänzung: Session-Grenze beim automatischen Abschluss

Start/Recovery und Abschluss koordinieren sich zusätzlich über ingest_gate.
Poll- und Simulatorabschluss tragen die konkrete Session-ID; ein verspäteter
Abschluss für A wird verworfen, wenn B aktuell ist. Prüfung, Persistenz und
Live-Veröffentlichung liegen innerhalb dieser Sperre. Interne locked-Helfer
vermeiden rekursives Sperren; öffentliche Abschlussaufrufe erwerben die Sperre.
Ein Regressionstest beweist, dass ein alter Abschluss B offen lässt und der
korrekt gebundene Abschluss B weiterhin schließen kann. Verbindungsupdates
prüfen ihre Worker-Generation unter dem Zustandslock.
