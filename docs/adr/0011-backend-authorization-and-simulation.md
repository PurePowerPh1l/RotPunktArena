# ADR: Backend-Rechte und Simulationsherkunft

Status: Accepted
Date: 2026-10-01

Die Admin-Sperre verhindert unberechtigte Verwaltungsaktionen auch bei direkten
WebView-IPC-Aufrufen. Sie ist eine Produktsperre auf einem vertrauenswürdigen
lokalen PC; sie schützt nicht vor einem OS-Administrator, der DB/Programmdateien
verändert. Die Freigabe ist pro Prozess und wird bei Restore/Reset wieder gesperrt.
Die UI öffnet bei Verwaltungsaktionen den bestehenden Auth-Dialog; Rust prüft
unabhängig davon vor jedem privilegierten Schreibpfad `AdminSession.require()`.

Schreibcommand-Inventar:

| Kategorie | Commands | Backend-Policy |
|---|---|---|
| Personen | create_person, update_person, delete_person, set_person_archived, promote_training_shooter | Admin |
| Wettkämpfe | create_competition, update_competition, set_competition_status, create_from_competition, set_competition_team_settings | Admin |
| Starter | add_entry, reorder_entries, set_entry_status, remove_entry, clone_entries | Admin |
| Teams | create_team, rename_team, set_team_archived, remove_team, add_team_member, remove_team_member, add_team_person, remove_team_person | Admin |
| Datenverlust | clear_training_history, restore_db_backup, reset_all_database | Admin |
| Injektion | queue_sim_shot, fire_aim_shot, set_auto_fire, dev_inject_test_shot | Admin + offene persistierte Simulatorsession ohne Wettkampf |
| Bedienablauf | start_training, start_entry_session, end_training, finish_probe, reset_training_series, set_training_endless, set_training_series_shots, save_training_session, resume_session, close_interrupted_session | Operator; Session-/Domain-Gates im Backend, keine Stammdaten-/Regelbearbeitung |
| Einstellungen | set_ui_prefs | Operator; ausschließlich validierte UI-Präferenzen |
| Geräte | rfcomm_setup_scan, rfcomm_setup_connect, rfcomm_forget_target, rfcomm_forget_device, rfcomm_reconnect, rfcomm_cancel_connect, rfcomm_open_pairing_settings | Operator; Geräte-/Verbindungsverwaltung |
| Sicherung/Export | create_db_backup, export_diagnostics, export_emergency_bundle | Operator; kontrollierte App-Data-Zielpfade |
| Auth | setup_admin_password, verify_admin_password, lock_admin_session | eigener Credential-Vertrag; Setup nur einmal |
| Testfreigabe | dev_unlock_admin_session | ausschließlich Debug-Build; Release lehnt ab |

Migration 17 ergänzt `sessions.simulated` (0/1). Neue Sessions setzen die Herkunft
im Start-Commit; Simulator-Wettkampfstarts werden abgewiesen. Recovery übernimmt
ausschließlich denselben gespeicherten Modus. Simulatorschüsse tragen zusätzlich
`actor_type=simulator`. Simulatortraining bleibt in der DB nachvollziehbar, wird
sichtbar markiert und nicht als Trainingsstatistik gespeichert.

Historische Sessions haben keine verlässlich rekonstruierbare Transportherkunft.
Die Migration klassifiziert diese nicht nachträglich als simuliert; sie gelten für
Recovery als Hardware und bestehende Ergebnisse bleiben unverändert. Ein alter
Simulatorlauf muss geschlossen und als explizites Simulatortraining neu gestartet
werden. Alte App-Versionen ignorieren das neue Feld und sind nach Migration kein
sicherer Rückfall für die neue Simulations-Policy; vor Update DB-Backup erstellen.

Regressionen: 30 gesperrte Commands über den echten Tauri-IPC-Dispatcher, erfolgreiche
freigegebene Mutation, Injektion gegen Hardware-/Wettkampf-/geschlossene Sessions,
persistierte Simulatorherkunft und Statistik-Ausschluss. Die IPC-Tests verwenden
Tauris MockRuntime; Windows-Testprogramme benötigen Common Controls v6 im Manifest.
