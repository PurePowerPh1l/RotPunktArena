# CODE_GUIDELINES.md

## 1. Ein Schreibpfad pro Aggregat
Jede Tabelle hat genau eine Rust-Funktion, die sie schreibt.
Niemals das gleiche INSERT/UPDATE an zwei Stellen im Code duplizieren.
Bei Bedarf: gemeinsame private Hilfsfunktion extrahieren statt kopieren.

Autosave/Recovery: Der Fortschrittsmarker (`last_autosave_at` / `last_autosave_sequence`)
wird in derselben Ingest-Transaktion gesetzt — kein zweiter Schreibpfad, kein Backup-Timer.

## 2. Command-Module nach Domäne, nicht nach Zeitpunkt
commands/
  live.rs        Session, Schuss, Verbindung
  bureau.rs      Personen, Wettkampf, Startliste (Teams aktuell hier; Split nach `teams.rs` optional)
  training.rs    Trainingshistorie
  recovery.rs    Recovery-Gate + Diagnose-Export (Autosave-Marker)
  dev.rs         Diagnose, Testschuss
Kein neuer Command landet direkt in lib.rs — nur Re-Export/Registrierung.

## 3. Contracts vor Implementierung
Jeder Tauri-Command und jedes emittierte Event hat einen Typ in
packages/contracts, der von TS UND Rust genutzt/generiert wird.
Kein manuell dupliziertes Interface in zwei Sprachen ohne Codegen-Check.
(Übergang: Domain-Typen in `packages/domain` wortgleich zu Rust halten.)

## 4. Maximale Funktionslänge als Code-Smell-Signal
> 40 Zeilen oder > 3 Verantwortlichkeiten → Funktion aufteilen.
Eine Funktion, die DB-Zugriff, Validierung UND Event-Emission mischt,
wird in drei benannte Schritte zerlegt.

## 5. Keine impliziten Seiteneffekte in Getter-artigen Funktionen
`get_live_state` darf niemals schreiben.
Schreibende Funktionen tragen ein Verb, das Wirkung ausdrückt:
`start_session`, `ingest_raw_frame`, `finish_series_if_needed`.

## 6. Ein Migrations-Skript = eine fachliche Änderung
Keine Migration darf zwei unabhängige Features gleichzeitig einführen
(z. B. nicht Nachkauf UND Teams in einer Version).
Migrationsname beschreibt die Absicht, nicht die Versionsnummer.

## 7. Keine God-Files
Kein Rust- oder TSX-File über ~300 Zeilen ohne explizite Begründung
im Kommentar-Header, warum die Trennung fachlich nicht sinnvoll ist.

## 8. Tests folgen dem Schreibpfad, nicht der UI
Für jeden Ingest-Outcome (Accepted, Duplicate, ParseFailed, LimitReached)
existiert mindestens ein Integrationstest — unabhängig von UI-Änderungen.

## 9. Naming-Konsistenz zwischen Rust und TS ist Pflicht, nicht Kür
EventKind, Status-Enums und DTOs müssen wortgleich sein.
Ein Rename in Rust erzwingt im selben PR den Rename in TS.

## 10. Kein „stilles“ Business-Logic-Duplicate im Frontend
Punktberechnung, Limit-Prüfung, Wettkampf-/Team-Ranking existieren nur in Rust.
React darf Ergebnisse anzeigen, nicht nachrechnen.

**Erlaubt (bewusst UI-Gamification, kein Wettkampf-Ergebnis):**
Soft-XP, Liga-SR und Achievements unter `apps/desktop/src/training/` leiten
Anzeige-Metriken aus bereits gespeicherten `TrainingSessionSummary`-Zeilen ab.
Sie dürfen Schusswerte, Session-Limits oder Platzierungen nicht neu berechnen.
Wettkampf-Ränge kommen als `rankPunkte` / `rankTeiler` aus der API.

## 11. KI-Review
Vor Merge KI-generierter Diffs: [ai-review-checklist.md](./ai-review-checklist.md).
