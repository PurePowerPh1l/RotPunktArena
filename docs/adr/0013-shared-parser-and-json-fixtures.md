# ADR 0013: Gemeinsame Parser- und JSON-Fixtures

Status: angenommen, 2026-10-01. Befund F12.

TypeScript akzeptierte mit `parseInt` Zahlenpräfixe und erzeugte bei defekten
Feldern NaN. Beide Parser verlangen jetzt vollständige ASCII-Zahlenfelder;
Dezimalfelder erlauben höchstens einen Punkt mit Ziffern auf beiden Seiten.
Vorzeichen und Ganzzahlfelder bleiben kompatibel. Parser-Version ist
`reddot-stx-v2`; gespeicherte v1-Schüsse werden nicht neu interpretiert.

`fixtures/protocol.json` wird in beiden Sprachen geprüft. TypeScript bewahrt
defekte Rohframes in einem `parse_error`-Ereignis und setzt danach den Stream
fort. Sniffer-Replay zeigt den Fehler mit Bytes; Analyse bricht ausdrücklich
ab, statt defekte Frames still zu überspringen. Diese synthetischen Fixtures
sind kein Nachweis unbekannter Hardware-Header oder Retransmit-Semantik.

`fixtures/ui-prefs.json` prüft Rust-Serialisierung, Deserialisierung und den
TypeScript-Default einschließlich camelCase, Enums und explizitem null.
Weitere Live-Contracts gehören zur Revisionierung in F10. Keine neue
Codegenerierung oder zweite Schemasprache wird eingeführt.
