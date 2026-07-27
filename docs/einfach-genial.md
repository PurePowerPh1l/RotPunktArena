# Einfach = Genial — Evidenz, Grenzen, Abnahme

Basierend auf der Retrospektive zum Soft-Wake-Überbau und den späteren Hardware-/RFCOMM-Labs: Leitlinie, die verhindert, dass Fixes **mehr Mechanik statt richtige Diagnose** einbauen.

Verwandt: [ai-review-checklist.md](./ai-review-checklist.md), [bluetooth-connection-stack.md](./bluetooth-connection-stack.md), [incremental-lab-first](../.cursor/rules/incremental-lab-first.mdc) (Cursor-Regel).

---

## Zweck

Diese Regeln gelten für Produktcode, Hardware-/Systemintegrationen, Recovery-Logik, UI-Zustandsmaschinen, Diagnosecode und AI-generierte Änderungen.

> Ein Fix ist erst dann ein Fix, wenn seine Wirkung reproduzierbar gemessen, seine Grenze dokumentiert und sein Rückbau einfach ist.

## Die Kernregel in einem Satz

Jede neue Schicht muss ihre Existenzberechtigung gegen eine **einzige, bereits laufende, einfache Lösung** beweisen — nicht gegen ein Problem in der Theorie.

---

## 1. Ein Symptom, eine Hypothese

- Formuliere vor jeder Änderung ein beobachtbares Problem.
- Formuliere dazu genau eine prüfbare Ursache oder Hypothese.
- Ändere nicht gleichzeitig Retry, Timeout, Pairing, UI, Recovery und Persistenz.
- Wenn mehrere Maßnahmen nötig erscheinen, teste sie zuerst getrennt im Lab.
- Ein intermittierender Erfolg ist kein Beweis, dass mehr Retries helfen; er ist zuerst ein Hinweis auf Timing, Race oder Interferenz.

**Gut:**

```text
Problem: RFCOMM-Start erzeugt bei schlafendem Ziel gelegentlich Windows-Toast.
Hypothese: Release vor dem zweiten Connect verändert den Windows-SPP-Zustand.
Test: A/B-Lab, gleicher Bond, Toast aktiv erfassen.
```

**Schlecht:**

```text
RFCOMM ist instabil → mehr Retries, Hook dauerhaft aktivieren,
SDP versuchen, Release, Backoff und UI-Toast zugleich ändern.
```

---

## 2. Produktpfad und Lab trennen

- Produktcode enthält nur Verhalten, das einen klaren Nutzwert und eine bestandene Abnahme hat.
- Experimentelle Varianten gehören in ein Lab-Binary, Testscript oder Feature-Flag — nie verdeckt in den Happy Path.
- Ein Lab darf aggressiv, langsam oder diagnostisch laut sein; der Produktpfad nicht.
- Jeder Diagnose-Entry-Point trägt sichtbar eine Kennzeichnung:

```text
DIAGNOSE-ONLY — not product path
```

- Ein Lab-Ergebnis wird erst nach mehreren reproduzierbaren Läufen zur Produktregel.

**Regel:** Was nur „manchmal hilft“, bleibt Diagnose. Was zuverlässig, verständlich und abgenommen hilft, darf Produkt werden.

---

## 3. Automatik ist konservativ

Automatische Aktionen dürfen keinen schwer rückgängig zu machenden Seiteneffekt haben, außer dieser Effekt ist explizit als Hardwarevertrag dokumentiert und getestet.

| Aktion | Automatisch erlaubt? | Bedingung |
|---|---|---|
| Status lesen, prüfen, loggen | Ja | Nebenwirkungsfrei |
| Einen stillen Connect versuchen | Ja | Begrenzt, cancelbar, kein UI |
| UI öffnen oder Toast erzeugen | Nein | Nur durch bewusste Nutzeraktion |
| Pairing / Forget / Reset | Normalerweise nein | Ausnahme nur mit klarer Hardware-Evidenz |
| Retry-Schleife | Nein | Nur mit fester Obergrenze und Abnahme |
| Adapter-/Radio-Reset | Nein | Nur explizite Recovery-Aktion |

Wenn eine Legacy-Hardware einen aggressiven Ablauf verlangt, gilt zusätzlich:

- Nur auf eine persistierte, validierte Geräteidentität anwenden.
- Genau einmal pro Auslöser ausführen.
- Nicht rekursiv wiederholen.
- Bei Fail in einen ruhigen, handlungsfähigen Zustand wechseln.
- Keine fremden Geräte anhand von Namen, Enumeration oder Heuristiken verändern.

---

## 4. Ein Owner, eine Wahrheit

- Eine Ressource hat genau einen Owner: Socket, Prozess, Handle, Pairing-Vorgang, Persistenzdatei oder Timer.
- Nur der Owner darf öffnen, schließen, schreiben, Pairing auslösen oder Status final publizieren.
- UI, Sessions und Nebenmodule senden Commands; sie übernehmen niemals Ressourcenbesitz.
- Es gibt genau eine autoritative Statusquelle.
- Kein Modul darf Status „zur Sicherheit“ direkt überschreiben.

**Mindest-Invarianten:**

```text
Nur Owner darf Socket/Handle schließen.
Nur ein Connect- oder Pair-Vorgang läuft gleichzeitig.
Session-Ende beendet nicht automatisch den langlebigen Link.
Ein alter Vorgang darf keinen neuen Zustand publizieren.
```

---

## 5. Jede lange Operation ist abbrechbar

Jeder Vorgang mit IO, Sleep, Polling, Pairing, Scan oder Timeout braucht eine klare Abbruchsemantik.

- Jeder Lauf erhält `generation`, `operationId` oder ein gleichwertiges Token.
- Target-Wechsel, Setup-Pause, Forget und Shutdown machen alte Arbeit ungültig.
- Nach jedem Blocker prüfen: Ist dieser Vorgang noch aktuell?
- Alte Ergebnisse dürfen weder Persistenz schreiben noch Linked melden.
- Abbruch ist ein normaler Ausgang, kein „unmöglicher Sonderfall“.

**Muster:**

```rust
let generation = self.generation;

let result = worker_connect(target).await;

if generation != self.generation {
    return; // Ergebnis ist veraltet und wirkungslos.
}

self.apply_connect_result(result);
```

---

## 6. Zustände sind Produktverträge

- Statusnamen beschreiben, was Nutzer und UI wissen müssen — nicht interne Implementierungsdetails.
- Technische Phasen bleiben Diagnose, sofern die UI sie nicht wirklich benötigt.
- Jeder sichtbare Zustand hat eine eindeutige nächste Aktion.
- Fehler dürfen nie als „Suche läuft“ oder „Verbinde …“ kaschiert werden.
- Ein Zustand darf nicht mehrere widersprüchliche Bedeutungen haben.

| Produktzustand | Nutzerbedeutung | Nächste Aktion |
|---|---|---|
| `needsTarget` | Kein Gerät eingerichtet | Einrichten |
| `connecting` | Kontrollierter Vorgang läuft | Warten oder abbrechen |
| `linked` | Verbindung nutzbar | Session starten |
| `idle` | Nicht verbunden, aber Ziel bekannt | Verbinden/Reparieren |
| `needsPairing` | Kopplung konnte nicht hergestellt werden | Erneut verbinden |
| `faulted` | Lokaler oder schwerer Fehler | Grund sehen, Setup/Forget |

---

## 7. Identität vor Heuristik

- Persistierte Geräteidentität ist technisch eindeutig, etwa BD_ADDR, Seriennummer oder UUID.
- Anzeigename, Name-Hints, COM-Port oder Discovery-Reihenfolge sind niemals die primäre Identität.
- Automatische destruktive Aktionen dürfen nur die kanonische Identität verwenden.
- Eine Heuristik darf Kandidaten finden, aber nicht stillschweigend ein fremdes Ziel dauerhaft übernehmen.
- Bei mehreren plausiblen Kandidaten: Nutzer auswählen lassen.

**Regel:** Namen helfen beim Finden. IDs entscheiden über Besitz, Persistenz und destruktive Aktionen.

---

## 8. Diagnostik ist ein Produktbestandteil

Logs müssen nicht maximal umfangreich sein, sondern eine konkrete Ursache rekonstruierbar machen.

Jeder relevante Vorgang sollte enthalten:

```text
runId
operationId / generation
origin
target identity
Start- und Endzeit
Dauer
Schritt
Ergebnis
normalisierter Fehlercode
Folgezustand
Retry-/Cancel-Entscheidung
```

Für Hardware- und Windows-Probleme zusätzlich:

```text
OS-Version
Adapter / Treiberversion
Geräte-Firmware oder Hardware-ID
App-Commit
Feature-Flags
```

**Nicht loggen:**

- Unstrukturierte Wiederholungen ohne Run-ID
- Passwörter, PINs oder geheime Schlüssel
- „failed“ ohne Schritt, Fehlercode und Folgezustand
- Daten, die niemand auswertet

Diagnosecode braucht einen Consumer: Lab, Support-Workflow, JSONL-Auswertung oder konkrete Testmatrix. Ohne Consumer wird er entfernt.

---

## 9. Tests beweisen Invarianten

- Unit-Tests prüfen Policy und Zustandsübergänge.
- Fake-/Simulations-Tests prüfen Fehlerklassen ohne Hardware.
- Hardwaretests prüfen Timing, Windows-UI, echte Pairing-Zustände und reale Nebenwirkungen.
- Eine Matrix enthält immer Pass-Kriterien, nicht nur Testnamen.
- Ein sichtbarer Toast, PIN-Dialog oder fremder Seiteneffekt ist ein explizites Testfeld, kein Kommentar.

Für jeden neuen automatischen Ablauf testen:

- Erfolg
- erwarteter Fail
- Timeout
- Cancel
- Shutdown
- Doppel-Trigger
- Wechsel des Ziels
- bereits laufende Operation
- Persistenzfehler
- UI während Zwischenzustand
- Wiederholung über viele Läufe

---

## 10. Kein Patch mitten im Soak

- Ein Soak-Block läuft auf einer festgehaltenen Baseline.
- Während eines Blocks: keine Codeänderung, kein Rebuild, keine Adapter-/Treiberänderung und kein Konfigurationswechsel.
- Jeder Fail wird gezählt, gesichert und analysiert.
- Ein fehlgeschlagener Lauf wird nicht durch einen erfolgreichen Wiederholungslauf „überschrieben“.
- Erst nach Abschluss eines Blocks werden Fehlermuster gruppiert und eine neue Hypothese formuliert.

Soak-Meta enthält mindestens:

```text
Datum
Branch / Commit
Dirty-Tree-Status
ausgeschlossene lokale Änderungen
Windows-Build
Adapter und Treiber
Hardware-ID
Testziel
Abnahmegrenze
```

---

## 11. Commits sind Review-Einheiten

Ein Commit beantwortet genau eine Frage: Was wurde geändert, warum, und wie kann ich diese Aussage prüfen oder zurücknehmen?

- Produktlogik, UI, Lab, Diagnose und Docs nicht vermischen.
- Abhängige Infrastruktur kommt vor dem Consumer-Commit.
- Test- und Diagnose-Binaries werden zusammen mit ihrem Build-Eintrag committed.
- Hardware-Evidenz wird separat dokumentiert, ohne den bereits getesteten Produkt-Commit nachträglich umzuschreiben.
- Formatierungs- oder Whitespace-Fixes gehören in einen separaten Hygiene-Commit.
- Keine persönlichen Editor-Regeln ins Repository, außer sie sind bewusst Teamstandard.

**Gute Commit-Reihenfolge:**

```text
1. Produktvertrag
2. Hardware-Evidenz / Doku
3. Diagnose-API
4. Lab-Tool
5. UI-Führung
6. optionale Meta-/Editor-Regel
```

---

## 12. Review-Checkliste

Vor Merge oder Commit:

### Architektur

- Hat die Änderung genau eine Verantwortung?
- Gibt es einen einzigen Owner für jede kritische Ressource?
- Kann eine alte Operation einen neuen Status überschreiben?
- Kann ein Nutzerklick einen bereits laufenden Vorgang duplizieren?
- Ist jeder Seiteneffekt bewusst begrenzt?

### Produktverhalten

- Ist klar, was automatisch passieren darf und was Nutzeraktion erfordert?
- Endet jeder Fail in einem ruhigen, verständlichen Zustand?
- Verursacht der Hintergrundpfad keine Toasts, Dialoge oder überraschende UI?
- Ist eine destructive Aktion auf eine eindeutige Identität begrenzt?
- Gibt es einen klaren nächsten Schritt für den Nutzer?

### Diagnose und Tests

- Gibt es Run-ID, Dauer, Fehlercode, Schritt und Folgezustand?
- Gibt es einen Test für Erfolg, Fail, Cancel, Shutdown und Doppel-Trigger?
- Ist Hardware-Evidenz einem konkreten Commit zugeordnet?
- Sind Lab-Ergebnisse klar von Produktverhalten getrennt?
- Wurde bei intermittierenden Fehlern erst gemessen statt Mechanik ergänzt?

### Git und Review

- Baut jeder Commit für sich?
- Ist der Commit-Scope ohne Nebenänderungen?
- Sind UI, Lab, Diagnose und Produktcode getrennt?
- Ist `git diff --check` sauber?
- Ist der Arbeitsbaum nach Abschluss bewusst sauber?

---

## Merge-Gate

Nicht mergen, wenn eine dieser Fragen mit „Nein“ beantwortet wird:

1. Kann ich den konkreten Fehler mit Log und Test reproduzierbar belegen?
2. Ist klar, warum genau dieser Fix wirkt und welche Nebenwirkung er bewusst hat?
3. Ist der Ablauf begrenzt, abbrechbar und single-flight?
4. Ist die Nutzerwirkung verständlich und frei von unerwarteter Windows-/System-UI?
5. Lässt sich die Änderung als einzelner Commit reviewen oder zurücknehmen?

Zusätzlich bei neuer Robustheits-/Retry-Schicht (Soft-Wake-Lektion):

1. Welches **konkrete Log-Signal** zeigt, dass die bestehende Lösung nicht reicht?
2. Interferiert die Schicht mit Auth/Pairing/anderen Timern? Explizit geprüft?
3. Was kostet die Schicht im **Erfolgsfall** (Zeit, Zustände)?
4. Ab welchem Messwert wird sie **wieder entfernt**, falls sie nicht hilft?

> Wenn eine zusätzliche Schicht nicht messbar eine konkrete Fehlerklasse reduziert, kommt sie nicht in den Produktpfad.

---

## Anhang: Soft-Wake-Gebote (Ursprung)

Die sieben Kurzregeln aus der Soft-Wake-Retrospektive — weiterhin gültig und in den Abschnitten oben ausgearbeitet:

1. Ein Symptom, eine Hypothese, ein Test — nie drei Fixes gleichzeitig.
2. Neue Komplexität braucht einen Vorher-Nachher-Beweis, nicht nur eine plausible Geschichte.
3. Serialisieren vor Intensivieren.
4. Wenn der manuelle Pfad funktioniert, kopiere ihn — bau keinen neuen Parallel-Stack.
5. Jede zusätzliche Zustandsstufe kostet Zeit im Erfolgsfall — rechne das immer mit.
6. Ein Rollback-Kriterium vor dem Ausbau festlegen, nicht danach.
7. Bei Unsicherheit: weniger Code gewinnt automatisch, bis das Gegenteil bewiesen ist.
