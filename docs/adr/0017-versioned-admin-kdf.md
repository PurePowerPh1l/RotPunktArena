# ADR 0017: Versionierte Admin-KDF und Versuchsdrosselung

Status: angenommen, 2026-10-01. Befund F14, begrenzter Teil umgesetzt.

Neue Passwörter werden als Record-Version 2 mit Argon2id v19, 19456 KiB,
zwei Durchläufen, Parallelität 1 und 32 Byte Ergebnis gespeichert. Die PHC-
Darstellung enthält die Parameter. Beim Lesen werden nur diese kanonischen
Parameter akzeptiert, bevor die KDF ausgeführt wird; manipulierte enorme
Kosten verursachen damit keine Speicheranforderung. Implementierung:
[RustCrypto Argon2 0.5.3](https://docs.rs/argon2/0.5.3/argon2/).

Version 1 (SHA-256 mit Salt) bleibt ausschließlich für Legacy-Verifikation
lesbar. Nach korrektem Passwort wird Version 2 gespeichert, bevor die Sitzung
entsperrt wird. Fehlversuche und Schreibfehler ändern die alten Credentials
nicht. Bestehende kurze Passwörter bleiben für die Migration verwendbar;
Neueinrichtung verlangt mindestens acht Unicode-Zeichen, maximal 1024 Bytes.
Salt stammt wie bisher aus einer zufälligen UUID v4 (122 Zufallsbits).

Ein Mutex serialisiert Passwortprüfungen je Prozess. Fünf Fehlversuche sperren
weitere Prüfungen 60 Sekunden, gemessen mit monotonic Instant. Erfolgreiche
Prüfung oder abgelaufene Sperrfrist setzt den Zähler zurück. Ein App-Neustart
setzt die Drosselung zurück; das ist kein Schutz gegen einen lokalen
OS-Administrator mit Datei-/Prozesszugriff (siehe ADR 0011).

Tests prüfen neue KDF, Legacy-Migration, falsches Passwort, fehlgeschlagene
Migrationspersistenz, unzulässige Parameter, Zeichen-/Bytegrenzen und
Drosselungsablauf ohne echte Wartezeiten. Passwortänderung und ein bewusstes
Recovery-Verfahren mit UI gehören in einen separaten P2-PR; es gibt keinen
neuen versteckten Passwort-Reset. Bei Verlust nicht die Live-DB löschen:
gesicherten Datenbestand und autorisierte Wartung anhand dokumentierter
Betriebsabnahme verwenden. Alte Apps verstehen Version 2 nicht; Downgrade
benötigt die Vor-Update-Sicherung und darf neue Wertungen nicht verwerfen.
