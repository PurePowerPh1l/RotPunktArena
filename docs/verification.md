# Verbindlicher Prüfpfad

`npm ci`, `npm ci --prefix tools/sniffer`, anschließend `npm run verify`.
Dies ist derselbe Pfad wie Windows-CI. Verify umfasst Domain-/JSON-Contracts,
Live-Revisionsordnung, normalisierte Trainingsvergleiche, Protokoll-Fixtures,
Sniffer, vorhandene Frontend-Selftests und Mutations-/Access-Prüfungen,
TypeScript-/Vite-Build, Rustfmt, Clippy ohne Warnungen und Rust-Unit-/Integrationstests
mit den Produktionsfeatures simulator,serial,rfcomm. Es gibt kein Cargo-Workspace;
alle Cargo-Aufrufe verwenden das Desktop-Manifest. Die experimentellen
softwake-labs sind keine Produktionsfeatures.

Gezielte Prüfungen sind `npm run test:contracts`, `npm run test:training`,
`npm run test:sniffer`, `npm run protocol:test` und `npm run verify:rust`.
Der native Installer-Build ist getrennt: `npm run desktop:build`, siehe
app-update-release.md. Ein bestandener Vite-Build ist kein Installer-Nachweis.

Der bestehende Rustfmt-Rückstand wurde einmalig in einem separaten Commit
bereinigt, damit der verlangte Format-Gate auf dem gesamten Manifest nutzbar
ist. Clippy-Bereinigungen bewahren Transport- und Datenverhalten. Explizite
Ownership-Parameter an drei Arena/Poll-Grenzen sowie große, begrenzte
Diagnosewerte haben lokal begründete `expect`-Attribute; keine globale
Lint-Abschaltung und keine Test-Deaktivierung. Erwartungen selbst werden
geprüft, wenn die betreffende Warnung nach einer Änderung nicht mehr entsteht.

Manuelle Release-Gates bleiben separat zu belegen: echte Hardware mit
ACK/Retry und schnellem Session-Wechsel, Restore/Reset bei knappem Datenträger
und Windows-Sperren, Prozessabbruch/OS-/Stromausfall, WebView-Ereignisordnung
und Tastaturführung sowie signierter Installer, installierter Updatepfad
und Schema-Rollback. Details und Befundstatus stehen in der Hardening-Abnahme.
