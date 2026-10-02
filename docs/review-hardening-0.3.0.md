# Review-Umsetzung 0.3.0

Diese Lieferung behandelt F01–F13 und die konkreten Betriebs-/UX-Ergänzungen des Repository-Reviews. Tests und praktische Betriebsabnahmen werden auf ausdrücklichen Nutzerwunsch nicht ausgeführt. Kompilierung, Produktionsbuild und Signatur-/Manifestprüfung sind eigene Release-Nachweise.

## Zustands- und Ownership-Vertrag

| Operation | Besitz / Barriere | Reihenfolge |
|---|---|---|
| Sessionstart / Wiederaufnahme | Engine lifecycle_gate → ingest_gate | persistierte Session und Regeln → aktuelle Session-ID + Live-Revision |
| Trainingslimit / Endlosmodus | ingest_gate → DB → inner | ein persistierter Konfigurationsschritt → Projektion → gegebenenfalls Abschluss derselben Session |
| Präferenzen | ingest_gate → DB → inner | UI-Prefs speichern → Enginepräferenz |
| Schuss | ingest_gate → SQLite immediate transaction → inner | Frame/Ereignis/Schuss/Summen/Recovery-Marker committen → ACK/Projektion |
| Setup-Suche | Connection-Owner | Pause mit neuer Generation bestätigen → SetupProgress nur für aktuelle Generation |
| Verbindungsstatus | ein SharedState-Lock | kohärenter, schreibfreier Snapshot |
| Update / Neustart | lifecycle_gate → Worker join → ingest_gate → Snapshotbarriere | offene Sessions abweisen → geprüfte Sicherung → neue Sessions sperren |
| Personenänderung / Promotion | SQLite immediate transaction | alle Schritte gemeinsam committen oder zurückrollen |

„Ein Schreibpfad“ meint einen fachlichen Pfad, nicht eine einzige physische SQLite-Verbindung. Engine, Ingest und Snapshot-Worker verwenden unterschiedliche Verbindungen. SQLite und die Barrieren koordinieren sie. Lockreihenfolge: lifecycle_gate vor ingest_gate; danach DB und inner. Schüsse sind unveränderlich; INSERT-/DELETE-Trigger pflegen Gesamtaggregate. Neue Schusskorrekturen benötigen einen Vertrag zur Neuberechnung der laufenden Summen.

## Migrationen und Kompatibilität

- Schema 19 speichert `sessions.endless`. Alt-Training ohne Limit und ohne training_saved wird als endlos behandelt. Bereits gespeicherte Historie wird nicht entfernt. Recovery verwendet gespeicherten Modus und Limit.
- Schema 20 führt `session_totals` und laufende Summen je Schuss ein. Vorhandene Schüsse werden einmalig nach Session, Klassifikation und Index aggregiert. Ingest liest danach konstante Summen.
- Keine automatische Rückwärtsmigration. Für einen Downgrade die vor dem Update erstellte SQLite-Sicherung mit der passenden Altversion wiederherstellen. Migrierte Datenbanken nicht mit älteren Clients beschreiben.
- Parser `reddot-stx-v3`: Ringwert 0,0 bis 10,9 einschließlich; Distanz nichtnegativ. Zahlentokenformat und Koordinaten bleiben unverändert. Ungültige Frames bleiben diagnostisch nachvollziehbar.
- Browserziele liegen unter `reddot.trainingGoals.v2:`. Leistungsziele verwenden zehn Schüsse. v1-Werte bleiben erhalten und werden als „Altziel: Zielwert prüfen“ markiert; ursprüngliche Serienlängen lassen sich nicht rekonstruieren.

## IPC-/Eventinventar

`packages/domain` ist der geltende Übergangsvertrag; `packages/contracts` existiert nicht. Rust serialisiert camelCase. Die neuen Live-Felder sind additive Änderungen zu Contract 1.

| Vertrag | Form |
|---|---|
| get_live_state / live_state | shotCount = gesamte aktive Klassifikation; shots = letzte höchstens 500 |
| shot | persistierter Index, validierte Zahlen, Gesamtsummen |
| get_session_shot_page | sessionId, probe, before exklusiv → höchstens 500 UiShots, aufsteigend |
| list_training_history | Filter + limit≤200 + offset≥0 → chronologische Seite |
| get_training_lifetime | je Person/Name Mengen und SR über alle gespeicherten Serien; vorheriger Zustand für Pulse |
| prepare_app_update / finish_app_update | Startschutz, Sicherung, Workerabschluss / Freigabe |
| get_backup_health / retry_snapshot | Zeitpunkt, Fehler, Queue-Drops, Speicherbelegung / neuer Auftrag |
| change_admin_password | Adminfreigabe + aktuelles/neues Passwort; anschließend erneut anmelden |
| export_personal_backup | versionierte Ziele; Admin erforderlich; Ergebnis wie Diagnoseexport |

Live-Snapshots bleiben nach verlorenen Events die Autorität. Live-Speicher, Trefferbild und Listenrendering sind begrenzt; ältere Schüsse sind paginiert erreichbar. Der Druck langer Live-Sessions kennzeichnet die letzten 500 Schüsse als Auszug. Gespeicherte vollständige Serien bleiben vollständig druckbar. Liga und XP basieren auf allen gespeicherten Serien; Trends, Leistungsziele und Erfolge auf dem bezeichneten 200er-Fenster.

## Backup und Diagnose

Der Settingsstatus zeigt Zeitpunkt, Fehler, Queueverluste und Platzbedarf von DB/Sicherungen/Exporten. Manuelle Sicherungen unterliegen nicht der automatischen Löschung. Retry stellt einen neuen Snapshotauftrag ein. Die bestehenden globalen Retentionsgrenzen bleiben maßgeblich.

ZIPs verwenden eindeutige temporäre Dateien und create-only-Veröffentlichung. DB, Manifest und Eventdump stammen aus derselben Snapshotdatenbank. Namenskollisionen erhalten den vorherigen Export. Temporäre DB-/WAL-/SHM-/ZIP-Dateien werden bestmöglich bereinigt.

Diagnose und Gesamtbackup enthalten personenbezogene Daten **und den Admin-Passworthash**. Nur vertrauenswürdig weitergeben. Das Gesamtbackup enthält `reddot.sqlite`, `manifest.json`, `rfcomm_devices.json`, `training-goals.json` und gegebenenfalls `events.jsonl`. Geräte und Browserziele sind separate Speicher ohne gemeinsamen SQLite-Snapshotzeitpunkt. Exportiert werden ausschließlich App-Trainingsziele.

Wiederherstellung: ZIP in ein neues Verzeichnis entpacken, Original behalten. DB über den vorhandenen Admin-Restorepfad aus backups einspielen. App schließen und Gerätegedächtnis im App-Data-Verzeichnis übernehmen. Ziele anhand ihrer versionierten Schlüssel im lokalen WebView-Speicher wiederherstellen; vorhandene Werte bewusst prüfen. Es gibt keinen automatischen ungeprüften Import.

## Passwort-Recovery und Vertrauensmodell

Normaler Weg: Einstellungen → Admin → aktuelles und neues Passwort. Der Backendpfad prüft die Freigabe und das aktuelle Passwort, drosselt Fehlversuche und sperrt nach erfolgreicher Änderung.

Vergessenes Passwort: nur Offline-Recovery durch den Besitzer des Windowskontos. Die Release-App besitzt keinen IPC-Bypass. Alle Appinstanzen schließen, dann mit Python 3 aus dem Repository:

```powershell
python tools/recover-admin.py "$env:APPDATA/de.disag.rotpunktarena/reddot.sqlite" --reset-admin
```

Der gewählte Pfad muss bereits existieren. Das Werkzeug prüft Appschema und Integrität, erstellt vorher eine eindeutige geprüfte SQLite-Vollsicherung und entfernt ausschließlich `settings.admin.auth`. Danach App starten und sofort ein neues Passwort setzen. Personen, Schüsse und Wettkämpfe bleiben erhalten. Das Backup enthält den alten Hash und ist vertraulich. Das OS-Konto bildet die Vertrauensgrenze.

## Verbindliche Regeln

1. Fachzustand erst nach DB-Commit veröffentlichen.
2. Sessionübergänge an die betroffene Session-ID binden.
3. Ingest-relevante Konfiguration unter ingest_gate ändern.
4. Mehrstufige Fachoperationen in einer Transaktion ausführen.
5. Connection-Owner bleibt alleiniger Statusschreiber.
6. Gerätenamen sind Suchhinweise, keine Authentifizierungsidentität.
7. Lange IPC-Aktionen auf Blocking-Worker verschieben.
8. Update und DB-Wartung koordinieren Sessions, Worker und Snapshots.
9. Recovery stellt persistierten Modus und Speicherentscheidung wieder her.
10. Eingaben syntaktisch und fachlich validieren; fehlerhafte Frames nicht werten.
11. Kritische DTOs im selben Commit in Rust, TS und Fixtures ändern.
12. Produktlogs in App-Data begrenzen; Tail liest höchstens 64 KiB.
13. Leistung auf vergleichbarer Schussbasis darstellen; Menge separat benennen.
14. Modale Dialoge besitzen Fokusgrenze, inert-Hintergrund und Fokusrestore.
15. Release-Nachweise nennen die tatsächlich ausgeführten Prüfungen.

## Reproduzierbarer Releasepfad

Produkt: `npm ci`, `npm run desktop:build` mit simulator,serial,rfcomm und dem bestehenden geheimen Signierschlüssel. Cargo-Jobs sind auf zwei begrenzt; speicherarme Hosts können CARGO_BUILD_JOBS=1 wählen. Lab-Binaries werden nicht im Installer ausgeliefert.

Signierte Installer und .sig in ein neues Releaseverzeichnis kopieren; latest.json mit tatsächlicher Signature und endgültiger URL erstellen. `node tools/verify-release-signatures.cjs apps/desktop/src-tauri/tauri.conf.json <Releaseverzeichnis> 0.3.0` prüft beide Signaturen und Trusted Comments gegen den unveränderten App-Public-Key. GitHub-Upload-Digests vor Veröffentlichung mit lokalen SHA256 vergleichen. Keine Secrets, Buildtargets oder fremde lokale Änderungen committen.

Ausdrücklich offen für diese Lieferung: Tests, Simulator-/Hardware-Abnahme, Lastmessungen, gerenderte Fokus-/Tastaturabnahme, Stromausfall sowie installierter Alt→Neu-Update-/Rollbackdurchlauf. Ein Build ersetzt diese Abnahmen nicht.
