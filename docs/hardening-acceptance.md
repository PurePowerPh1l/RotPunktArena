# Hardening-Abnahme F01–F14

Stand: 2026-10-01, Basis main 42c53bb (Version 0.1.23).
Branch: `codex/reliability-security-hardening`.

## Befunde und Nachweise

Alle Befunde wurden am Ausgangscode bestätigt; keiner wurde nur wegen der
Review behauptet oder als bereits erledigt markiert. Historische Schusswerte
werden nicht neu berechnet. Tests verwenden ausschließlich erzeugte Temp-DBs
und synthetische Personen-/Gerätedaten.

| Finding | Status | Ursache / Nachweis | Dokumentation |
|---|---|---|---|
| F01 | umgesetzt | SQLite-Backup-Transaktion, geprüfte Staging-/Rollback-DB; ungültig/neueres Schema, Teilkopie, Postprüfung, Rollback-Dateifehler, verzögerter Worker und vollständige Reset-Barriere getestet | ADR 0010 |
| F02 | umgesetzt | 30 gesperrte Commands im echten IPC-Dispatcher; freigegebene Mutation; Herkunft persistiert, Recovery-Modus geprüft, Hardware-/Wettkampf-Injektion abgewiesen | ADR 0011 |
| F03 | umgesetzt | Recovery-Abschluss gibt Inner-Guard vor Snapshot frei; Timeout-Regression | ADR 0006 |
| F04 | umgesetzt | eindeutige Sink-Leases mit Register-/Unregister-Antwort; alter Close/Drain, doppelte/veraltete und abgebrochene Registrierung getestet | ADR 0007 |
| F05 | umgesetzt | IMMEDIATE-Commit von Phase und Event; ingest_gate bis Projektion/Benachrichtigung; konkurrierender erster Wertungsschuss und Triggerfehler getestet | ADR 0008 |
| F06 | umgesetzt | Abschlussfehler werden propagiert; Commit vor Erfolg, idempotenter Retry; verspäteter Abschluss A kann B nicht schließen | ADR 0006 |
| F07 | umgesetzt | IMMEDIATE-Transaktion, aktive direkte/Entry-Referenzen abgewiesen; Fehler an acht Mutationsstellen mit vollständigem Rollback | ADR 0005 |
| F08 | umgesetzt | WAL/FULL auf jeder Verbindung, Reopen-Test und 200 tatsächliche Arena-Commits pro Modus gemessen | ADR 0009 |
| F09 | umgesetzt | Migration 18 speichert Regeln, alte Sessions explizit legacy_current; Regel-/Teamänderungen ab erstem Start blockiert; gespeicherte Werte bleiben gleich | ADR 0012 |
| F10 | umgesetzt | Contract v1, Session/Phase/Revision, Subscription vor Snapshot, vollständige Projektionen und 5s-Resync; positive/negative gemeinsame JSON-Fixtures | ADR 0015 |
| F11 | Kern umgesetzt; P2-Rest ausgelagert | Queue 16, besitzender Worker, Shutdown/Restore-Barriere; global 100 Dateien/30 Tage/512 MiB; Windows-Publikationsfehler, Retention und Export-Überschreiben getestet | ADR 0014; Follow-up A |
| F12 | umgesetzt | zwölf gemeinsame Parser-Fixtures, volle Zahlenvalidierung, Parser v2; Domain/Live/Sniffer/Rustfmt/Clippy im Root-Verify und Windows-CI | ADR 0013; verification.md |
| F13 | Vergleich umgesetzt; P2-Rest ausgelagert | Zehnerskala für Liga/Transfer/Trend/Chart, gewichteter Teiler; gleiche Leistung 5/10/20/30; gemeinsames 200er-Fenster ausdrücklich angezeigt | ADR 0016; Follow-up B |
| F14 | KDF/Drosselung umgesetzt; P2-Rest ausgelagert | Argon2id Record v2, Kosten fest, Legacy-Migration vor Unlock; Fehlpasswort/Schreibfehler/Parameter-/Zeitgrenzen getestet | ADR 0017; Follow-up C |

## Datenmodell, Lifecycle und Rollback

Migration 17 ergänzt Simulatorherkunft. Historische Transportherkunft wird
nicht erfunden; alte Sessions behalten ihre Ergebnisse und werden für Recovery
als Hardware behandelt. Migration 18 hält die aktuell vorhandenen Regeln als
gekennzeichnete Legacy-Baseline fest. Frühere Regeländerungen können nicht
rekonstruiert werden. Neue Sessions erhalten captured-Regeln im Start-Commit.

Lock-Reihenfolge: lifecycle_gate (Start/Recovery/Wartung), ingest_gate,
Engine-DB, inner. Poll-Ingest hält ingest_gate vom Commit bis zur Projektion;
Wartung signalisiert Stop und joint alte Worker vor dem DB-Austausch.
Snapshot-I/O besitzt eine eigene Generationsbarriere; alte Aufträge werden
nach Restore/Reset nicht gegen die Ersatz-DB ausgeführt.

Die Live-Datei und SQLite-Verbindung bleiben beim Restore erhalten. Ersatz
wird vorbereitet, migriert und validiert; atomare SQLite-Kopie kann intern
zurückrollen. Eine zusätzliche validierte Datei `restore-rollback-<uuid>.sqlite`
bleibt neben der Live-DB erhalten. Fehler nach dem Austausch stellen sie
wieder her; doppelte Fehler nennen ihren Pfad ausdrücklich. Rollback-Dateien
werden nicht automatisch entfernt und sollten im Betrieb kontrolliert
aufbewahrt werden. Restore/Reset sperrt anschließend die Admin-Sitzung.

WAL/FULL sichert Commits gemäß SQLite gegen Prozess-/OS-/Stromausfälle ab,
sofern Dateisystem und Datenträger Sync korrekt umsetzen. Der Windows-Debug-
Messlauf ergab NORMAL Median 0,189 ms / p95 0,381 ms, FULL Median 1,357 ms /
p95 2,535 ms. Dies ist keine reale Stromausfall-Abnahme oder Hardware-Latenz.
Details und Grenzen: [ADR 0009](adr/0009-shot-durability.md).

Downgrade ist keine sichere Policy-Rücknahme: alte Apps ignorieren neue
Simulator-/Regel-Guards und verstehen KDF v2 nicht. Vor Update eine validierte
DB-Sicherung erstellen. Eine Rückkehr auf die Vor-Update-DB darf neue Wertungen
nicht unbemerkt verwerfen; getrennt exportieren und kontrolliert abgleichen.
DB-Backups enthalten weder den separaten Gerätespeicher noch Browser-Trainingsziele.

## Automatisierte Prüfung und Produktionsbuild

Der vollständige erweiterte Root-Verify umfasst [verification.md](verification.md).
Vollständiger Lauf einschließlich finaler Contract-Fixtures und Reset-Barriere: 164 Rust-Unit-,
15 Arena-Integrations- und 6 Session-Grenztests, sämtliche TS-/Sniffer-Prüfungen,
TypeScript/Vite, Rustfmt und Clippy ohne Warnungen. Das Ergebnis wird im PR
festgehalten. Cargo kann beim Linken eine rein informative MSVC-Ausgabe zur
Erzeugung der DLL-Importbibliothek als linker_messages-Warnung anzeigen;
die separate strenge Clippy-Prüfung besteht.

`npm run desktop:build` erzeugte die optimierte App sowie MSI und NSIS für
0.1.23. Der Befehl endete bei der Updater-Signierung mit Fehler: kein gesetzter
TAURI_SIGNING_PRIVATE_KEY. Die Artefakte sind keine neue freigegebene Version
und werden nicht veröffentlicht. Kein lokaler Lab-Schlüssel wurde als
Produktionsschlüssel übernommen. Autoritative Version bleibt 0.1.23 bis die
Release-Gates erfüllt sind. Ein späteres Release benötigt konsistenten Bump,
sauberen Checkout und erneuten signierten Build nach app-update-release.md.

## Offene manuelle Gates: kein Release

- [ ] Reale DISAG-Hardware: Probe/Wertung, identische Schüsse vs. Retransmits,
  ACK/Retry nach Persistenzfehler, schneller Session-Wechsel mit alter Verbindung.
- [ ] Restore/Reset auf realem Windows-Datenträger mit wenig freiem Platz und
  Sperren; Strom-/OS-Abbruch an Commit-, Snapshot- und Austauschgrenzen; Neustart
  anhand des Live- oder Rückfallstands. Automatisierte Windows-Sperrtests ersetzen
  diese Betriebsabnahme nicht.
- [ ] Reale ältere Vereins-DB als Kopie: Migrationen, Ergebnisse vorher/nachher,
  historische Simulator-/Regelunsicherheiten prüfen; Original unverändert lassen.
- [ ] WebView: verzögerte Events/Antworten, Probe-/Session-Wechsel, Wiederaufnahme,
  sichtbarer Abschlussfehler, Tastatur/Fokus, Simulatorhinweis und Admin-Prompts.
- [ ] Autorisierter Produktions-Signing-Key, signierter Installer und korrektes
  latest.json; keine Debug-Daten/Secrets im Paket; installierte Altversion → Neu,
  Offline-/Bad-Manifest-/Bad-Signature-Fälle und bewusstes Schema-Rollback.

Nicht ausgeführte P0-Betriebsabnahmen und fehlende Signierung sperren das Release.
Der PR enthält diese Grenzen. Merge benötigt zusätzlich grüne Remote-CI und
sämtliche vom Auftrag definierten Merge-Gates; kein Gate wird pauschal angenommen.

## Klar abgegrenzte P2-Follow-ups

A — Sicherungsbetrieb: UI für Zeitpunkt/Größe/Ergebnis/Retry, rotierendes
App-Data-Log, Last-/Speichermessung für viele Sessions und Endlosbetrieb.
Abnahme: Queue-Überlast und Fehler sichtbar, lange Historie bleibt aus SQLite
abrufbar; begrenzte Live-Projektion erst nach Messung. Siehe ADR 0014/0015.

B — Vollständige Trainingsstatistik: serverseitige Lifetime-Aggregate,
paginierte Historie und stabile Liga je Schütze. Abnahme mit mehr als 200
Serien und seltenen Schützen; Ziele/Pulse/Rivalen auf kompatible Längen prüfen.
Siehe ADR 0016. Die aktuelle UI sagt ausdrücklich, dass sie ein Fenster zeigt.

C — Administration: UI für Passwortänderung mit erneuter Prüfung und
kontrolliertes Recovery-Verfahren. Abnahme von falschem altem Passwort,
Commitfehler, Credentials-Wechsel und Wiederanlauf; kein verdeckter Bypass.
Siehe ADR 0017.

D — Weitere Produktideen der Review: Update/Neustart an offene Sessions
koordinieren, versionierten selektiven Gesamtexport für DB/Geräte/Ziele und
isolierte Ranking-Funktionen bei belegtem Wartungsnutzen. Eigene PRs mit
Domain-/Interleaving-Tests; keine ungemessene Neuarchitektur im Hardening-PR.
