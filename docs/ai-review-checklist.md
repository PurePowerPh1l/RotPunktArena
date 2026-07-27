# KI-Review-Checkliste

Kurze Gegenregeln für Review von KI-generierten Diffs — nicht nur „läuft es?“, sondern „passt es ins Muster?“.

| KI-Antimuster | Gegenregel |
|---|---|
| Neue Hilfsfunktion statt vorhandene wiederzuverwenden | Vor jedem neuen Snippet: gibt es das schon in `db/*.rs`, `engine/` oder Shared Hooks? |
| Neuer Command für Variante statt Parametrisierung | Erst prüfen, ob ein bestehender Command einen optionalen Parameter bekommen kann |
| Copy-Paste zwischen ähnlichen React-Views | Gemeinsame Hooks/Components extrahieren, sobald sich Logik zum dritten Mal wiederholt |
| Übermäßige Kommentare statt klarer Namen | Code soll durch Namen erklärend sein; Kommentare nur für „warum“, nicht „was“ |
| Stillschweigende Erweiterung bestehender Funktionen um Sonderfälle | Neue fachliche Fälle bekommen eine eigene benannte Funktion statt `if special_case` in bestehender Logik |
| Große PRs mit UI + DB + Migration + Command gleichzeitig | Ein PR = eine Schicht oder ein klar abgegrenztes Feature-Slice |
| Mehr Robustheitsschichten ohne A/B (ACL/Verify/Stabilize/…) | [Einfach = Genial](./einfach-genial.md): eine Hypothese, Lab≠Produkt, Serialisieren vor Intensivieren, Merge-Gate |

Zusätzlich immer prüfen:

1. **Ein Schreibpfad** — kein zweites INSERT/UPDATE für dieselbe Tabelle (siehe [code-guidelines.md](./code-guidelines.md)).
2. **Contracts wortgleich** — `EventKind`, Status-Enums, DTO-Felder zwischen Rust und `packages/domain`.
3. **Kein stilles Business-Logic-Duplicate** im Frontend (Punkte, Limits, Wettkampf-Ranking nur in Rust; UI-Gamification unter `training/` ist dokumentierte Ausnahme).
4. **Recovery/Export** und andere Absicherung vor dem nächsten Feature-Slice.
