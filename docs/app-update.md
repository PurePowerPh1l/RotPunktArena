# App-Update (V1/V2) — Produktvertrag, Analyse, Slice-Plan

**Status:** Planung (Slice 0) — keine produktive Updater-Implementierung in diesem Dokument-Commit.  
**Branch (Plan):** `plan/app-update-v1-v2`  
**Umsetzung:** nur nach explizitem GO, jeweils eigener Feature-Branch.

---

## Produktvertrag

| Regel | Bedeutung |
|---|---|
| Kein Auto-Check beim App-Start | Update-Suche nur auf Nutzerklick |
| Kein stilles Auto-Update | Kein Hintergrund-Download ohne Aktion |
| Kein Zwangsupdate | App bleibt ohne Update nutzbar |
| Nur Stable | Ein Kanal; kein Beta/Canary |
| Hosting-Ziel | GitHub Releases (Stable-Manifest + Artefakte) |
| Releases öffentlich | **Öffentliche** GitHub Releases (Stable). Private Releases sind Absicht: zusätzliche Auth-/ACL-Hürden für den Updater — nicht der Default-Pfad |
| Lab ≠ Produkt | Lokale Verifikation nur mit installierter Altversion + signierter Neuversion |
| Eine Versionsquelle | Maßgeblich: `apps/desktop/src-tauri/tauri.conf.json` → `version`. UI und Updater lesen die Tauri-Bundle-Version (`getVersion`). Cargo.toml / package.json bei jedem Release **mitziehen**, nicht als zweite Wahrheit |

---

## Zielbild (Scope)

### V1 (In-App)

- Aktuelle App-Version anzeigen
- Manueller Button „Nach Updates suchen“
- Ruhige Zustände: aktuell / verfügbar / Fehler

### V2 (In-App)

- Update herunterladen (nach Nutzeraktion)
- Installieren (nach Bestätigung)
- Neustart zum Anwenden anbieten

### Explizit nicht

- Auto-Update / Forced Update / Mehrkanal
- Fake-Demo-Flow ohne echte Signatur
- Voll-CI oder Hosting-Magie im ersten Produkt-PR
- Secrets / Private Keys im Repo

---

## A. Codebase-Analyse

Stand der Analyse: Workspace `Disag Reddot 2`, Tauri-Desktop-App unter `apps/desktop`.

### A.1 Frontend / Einstellungen

| Frage | Befund |
|---|---|
| Settings-Sheet | [`apps/desktop/src/components/SettingsSheet.tsx`](../apps/desktop/src/components/SettingsSheet.tsx) in [`SideSheetShell`](../apps/desktop/src/components/SideSheetShell.tsx) |
| Sections (Accordion, eine offen) | `allgemein` → `darstellung` → `arena` → `verbindung` → `backups` → `admin` (`SettingsSectionId`) |
| Default beim Öffnen | `"verbindung"` |
| Slot „App & Updates“ | Neue ID z. B. `app` **nach Allgemein, vor Darstellung** (produktnah, nicht Admin) |
| Primitives | [`settings/SettingsParts.tsx`](../apps/desktop/src/components/settings/SettingsParts.tsx): `SettingsSection`, `SettingsInfoRow`, `SettingsHint`, `SettingsStatusPill` — geeignet für Version/Status |
| Async-Muster | Settings: lokales `run`/`busy`/`error` (Backups). Shared: [`hooks/useAsyncAction.ts`](../apps/desktop/src/hooks/useAsyncAction.ts) (single-flight). Vorbild für `useAppUpdate`. |
| Toast/Dialog | Kein globales Toast. Inline `banner-error`; Bestätigungen via `window.confirm`/`alert` (wie Restore). Training-`MomentFlash` nicht nutzen. |
| Tauri-Bridge | Domain-Wrapper unter `apps/desktop/src/api/*`, Barrel [`api/commands.ts`](../apps/desktop/src/api/commands.ts). Neues Modul z. B. `api/updates.ts` — nicht Logik in `App.tsx`. |
| Version in UI | **Fehlt.** Kein About/Update-Bereich. Diagnose-Export nutzt serverseitig `env!("CARGO_PKG_VERSION")`; Frontend zeigt keine App-Version. |

**Empfehlung UI:** Section analog Verbindung/Backups — `SettingsInfoRow` (Version, Update-Status), Primary-Button, `SettingsHint`, Sheet-`error`/`busy`.

### A.2 Tauri / Desktop / Plugins

| Frage | Befund |
|---|---|
| Tauri-Version | Config Schema 2; Lock typisch **tauri 2.11.x**, `@tauri-apps/api` / CLI `^2` |
| Registrierte Plugins | Nur **`tauri-plugin-opener`** in [`Cargo.toml`](../apps/desktop/src-tauri/Cargo.toml) und [`lib.rs`](../apps/desktop/src-tauri/src/lib.rs) (`.plugin(tauri_plugin_opener::init())`) |
| Updater / Process | **Nicht vorhanden** (weder Rust-Deps noch npm-Deps noch Capabilities) |
| Capabilities | Eine Datei: [`capabilities/default.json`](../apps/desktop/src-tauri/capabilities/default.json) — `core:default`, `opener:default`, Window-/Event-Permissions |
| Plugin-Registrierung später | Sauber parallel zu opener in `lib.rs`; Permissions in `default.json` (z. B. `updater:default`, `process:allow-relaunch`) |
| `tauri.conf.json` Bundle | `bundle.active`, `targets: "all"`, Icons — **kein** `plugins.updater`, **kein** `createUpdaterArtifacts`, **kein** `pubkey` |
| Versionen (Drift) | Überall manuell **`0.1.0`**: `tauri.conf.json`, `src-tauri/Cargo.toml`, `apps/desktop/package.json`, Root-`package.json`, teils Packages. **Kein Sync-Script.** Regel: siehe Produktvertrag — `tauri.conf.json` ist maßgeblich. |

### A.3 Release / Distribution

| Frage | Befund |
|---|---|
| Build | Root: `npm run desktop:build` → Tauri build mit Features `simulator,serial,rfcomm` |
| Artefakte | README: Installer unter `apps/desktop/src-tauri/target/release/bundle/` (NSIS/MSI o. Ä. je nach Target) |
| CI | Kein `.github/`-Workflows-Verzeichnis |
| Release-Doku | Keine eigene Distribution-/Signing-Doku (nur Build-Hinweis) |
| Signing / Keys | Nicht im Repo (korrekt) |
| Hosting-Entscheidung | **Öffentliche GitHub Releases** als Stable-Endpoint für Manifest (`latest.json` o. Ä.) + Updater-Artefakte + `.sig`. Realistisch ohne bestehende Infra; Endpoint-URL in Config später austauschbar. Lokales HTTP nur Lab mit explizit dokumentierter unsicherer Dev-only-Config; Produktiv nur HTTPS. |
| Öffentlich vs. privat | Default: **öffentlich**. Private Releases erfordern typischerweise Auth-Token/ACL und komplizieren den Updater-Client — nur bei bewusster Produktentscheidung und eigener Slice-Arbeit, nicht stillschweigend. |

### A.4 Architekturziel (später, nicht jetzt)

```text
SettingsSection „App & Updates“
        │
        ▼
  useAppUpdate()     ← Statusmaschine, single-flight, kein Start-Check
        │
        ▼
  api/updates.ts     ← Bridge: getVersion / check / download+install / relaunch
        │
   ┌────┴────┐
   ▼         ▼
 core API   tauri-plugin-updater (+ process relaunch)
              │
              ▼
     GitHub Releases (Stable)
```

Trennung: UI · Hook/Service · Bridge/Plugins · Release-Infra (Keys, Manifest, Bump).

---

## B. Risiken / Unklarheiten

- **Updater-Signierung fehlt** — ohne Keypaar + `pubkey` in Config sind echte Checks/Installs nicht verifizierbar; Private Key darf nie ins Repo.
- **Key-Handling/Rotation unklar** — wer released, wo liegt der Private Key (CI-Secret / Passwort-Manager), wie rotiert wird.
- **Manifest-Hosting** — kein `latest.json`, keine Release-Pipeline; Endpoint ist Platzhalter bis zum ersten **öffentlichen** signierten Release. Tauri erwartet HTTPS in Prod. Private Releases = Extra-Auth, nicht Default.
- **Dev vs. installierte App** — `tauri dev` / ungepackte Builds sind kein Ersatz für Update-E2E; UI muss Dev ruhig abfangen („nur in installierter App“).
- **`createUpdaterArtifacts` + `.sig` → Manifest** — manuell fehleranfällig; falsche Sig → Check/Install bricht trotz „neuer Version“.
- **Relaunch** — braucht `tauri-plugin-process`; in `lib.rs` gibt es RFCOMM-Shutdown bei Window-Close — nach Relaunch beobachten, **keinen** neuen Auto-Reconnect-Pfad einbauen. Fallback: siehe Slice 3.
- **Versions-Drift** — mehrere `0.1.0`-Quellen ohne Sync. Ab Slice 1 gilt die Regel „eine Quelle“ (`tauri.conf.json`); Sync der Begleitdateien spätestens in Slice 4 Checkliste.
- **CI-Lücke** — kein Pipeline-Contract; Automatisierung erst nach manuell grünem Lab (Slice 4), sonst Scope-Explosion.
- **Authenticode ≠ Updater-Sig** — Windows SmartScreen ist getrennt; Out-of-Scope für V1/V2-Kern, kann aber Nutzerwahrnehmung beeinflussen.
- **SettingsSheet-Größe** — neue Section ok; Hook extrahieren, damit das Sheet nicht zum God-File wird.

---

## C. Slice-Plan (nur Planung — Umsetzung nach GO)

Jeder Slice: eigener Branch, eigener Commit, kein fachfremder Diff.

### Slice 0 — Analyse & Vertrag (dieser Auftrag)

| | |
|---|---|
| **Branch** | `plan/app-update-v1-v2` |
| **Ziel** | Entscheidungen und Testkonzept schriftlich fixieren |
| **In Scope** | Dieses Dokument (`docs/app-update.md`); optional Index-Eintrag in `docs/README.md` |
| **Out of Scope** | Plugins, Settings-UI, API, Keys, Release-Skripte |
| **Tests** | Review: Vertrag, Slices, Lab-Checkliste, Hosting-Entscheidung vollständig |
| **Commit** | `docs: App-Update Vertrag, Architektur und Slice-Plan` |

### Slice 1 — Version-UI (V1a)

| | |
|---|---|
| **Branch** | `feature/app-update-v1-version` |
| **Ziel** | Section „App & Updates“ zeigt aktuelle Version |
| **In Scope** | Section-ID `app` in SettingsSheet; dünnes `api/updates.ts` mit `getVersion` (Tauri-Bundle-Version = maßgeblich laut `tauri.conf.json`); `SettingsInfoRow`; CSS nur analog bestehend; in Doku/Kommentar festhalten: Anzeige = eine Quelle, kein paralleles Auslesen aus package.json |
| **Out of Scope** | Updater-Plugin, Check-Button-Netzwerk, Download, Signing, Endpoints; kein Sync-Script für alle Manifeste (kommt Slice 4) |
| **Tests** | Angezeigte Version = `getVersion` / `tauri.conf.json`; Section Accordion; kein Netzwerk; Rest-Settings unverändert |
| **Commit** | `feat(settings): App-Version in App & Updates anzeigen` |

### Slice 2 — Manueller Update-Check (V1b)

| | |
|---|---|
| **Branch** | `feature/app-update-v1-check` |
| **Ziel** | Button „Nach Updates suchen“ mit ruhigen Zuständen |
| **In Scope** | `tauri-plugin-updater` + Capability; `plugins.updater` (`pubkey`, Stable-Endpoint); `check()` nur auf Klick; `useAppUpdate` (`idle` / `checking` / `upToDate` / `available` / `error`); Dev: ruhiger Hinweis statt echter Update-Logik |
| **Out of Scope** | Download/Install/Relaunch; Auto-Check; Hintergrund-Download; Beta; CI |
| **Tests** | Siehe D — V1-Check-Fälle; kein Check beim App-Start; single-flight |
| **Commit** | `feat(settings): manuellen Update-Check ergänzen` |

### Slice 3 — Download / Install / Relaunch (V2)

| | |
|---|---|
| **Branch** | `feature/app-update-v2-install` |
| **Ziel** | Nach Bestätigung laden, installieren, Neustart anbieten |
| **In Scope** | Download + Busy/Progress; `window.confirm` vor Install; `tauri-plugin-process` relaunch; klare Fehlerzustände; lange Ops abbrechbar/single-flight; **Relaunch-Fallback:** schlägt Relaunch fehl → Nutzer klar zum **manuellen Neustart** auffordern; Install-Erfolg und „App läuft schon neu“ nicht vermischen — nie „Update erfolgreich angewendet“, wenn der Prozess noch die Alt-Session ist |
| **Out of Scope** | Silent/Forced Update; Voll-CI; Authenticode |
| **Tests** | Siehe D — V2-E2E und Negativfälle (inkl. Relaunch-Fail → manueller-Neustart-Hinweis) |
| **Commit** | `feat(settings): Update herunterladen, installieren und neu starten` |

### Slice 4 — Release-Checkliste

| | |
|---|---|
| **Branch** | `docs/app-update-release` |
| **Ziel** | Wiederholbarer Release-Pfad ohne Secrets im Repo |
| **In Scope** | Keygen offline; `createUpdaterArtifacts`; `.sig` → Manifest; Version-Bump-Regel (`tauri.conf.json` bumpen, Cargo.toml + package.json mitziehen); **öffentliche** GitHub-Release-Schritte (manuell, CI später) |
| **Out of Scope** | Vollautomatisierte Pipeline; Canary; Hosting-Umzug |
| **Tests** | Ein Lab-Durchlauf Alt → Neu gegen release-ähnliches Manifest |
| **Commit** | `docs: Release-Checkliste für signierte App-Updates` |
| **Ergebnis** | [`app-update-release.md`](./app-update-release.md) |

---

## D. Teststrategie (planen, nicht implementieren)

### Lab-Voraussetzungen (ab Slice 2 / zwingend Slice 3)

1. Updater-Keypaar **außerhalb** des Repos erzeugen; nur Public Key in Config.
2. Zwei Builds: installierte **Altversion**, neuere **signierte** Version.
3. `createUpdaterArtifacts` aktiv; `.sig` korrekt ins Manifest übernommen.
4. Manifest (`latest.json`) zeigt von Altversion auf Neu-Artefakt (HTTPS oder dokumentierte unsichere Lab-only-Config).
5. Verifikation **nur** an installierter App — nicht allein `npm run desktop:dev`.

### V1 — Version-UI (Slice 1)

| Fall | Erwartung |
|---|---|
| Dev | Version sichtbar; Wert konsistent mit Build/Config |
| Kein Netzwerk | Kein HTTP beim Öffnen der Section |
| Accordion | Section öffnet/schließt wie andere |

### V1 — Manueller Check (Slice 2)

| Fall | Erwartung |
|---|---|
| Aktuell | Status „auf dem neuesten Stand“ (o. Ä.) |
| Update verfügbar | Status zeigt verfügbare Version; noch kein Download |
| Manifest-/Netzfehler | Ruhiger Fehler im Sheet; App weiter nutzbar |
| Dev-Mode | Hinweis statt kaputter Plugin-Pfad |
| Double-Click | Single-flight; kein paralleler Zweit-Check |
| App-Start | **Kein** automatischer Check |

### V2 — Happy Path (Slice 3)

1. Altversion installiert, Manifest zeigt Neu.
2. Check → available (Start-Notice und/oder Settings).
3. Confirm → gemeinsames Update-Progress-Sheet mit Download-Balken.
4. Windows: `installMode: quiet` → stilles NSIS (`/S /R`); App beendet sich und startet neu — kein Installer-Fenster.
5. Nach Neustart: neue Version in „App & Updates“ sichtbar.
6. Fallback (nicht-Windows / kein Auto-Exit): `readyToRelaunch` + „Jetzt neu starten“.

### V2 — Negativ

| Fall | Erwartung |
|---|---|
| Kein Update | Kein Download-UI |
| Manifest kaputt / 404 | Fehlerzustand, idle danach nutzbar |
| Netz weg | Fehler, kein hängendes „Verbinde…“ |
| Signatur falsch | Abbruch mit klarer Fehlerklasse |
| Download/Install fail | Busy endet; kein kaputter Zwischenzustand |
| Confirm abbrechen | Bleibt bei available/idle ohne Install |
| Relaunch fail | Kein „Update erfolgreich angewendet“; klarer Hinweis **App manuell neu starten**; Zustand bleibt ehrlich (installiert, Neustart ausstehend) |

### Meta (wie Soak-Regel)

Bei Lab-Läufen notieren: Commit, Dirty-Tree, Adapter/OS, Hardware-ID falls relevant — Baseline nicht mit Patches überschreiben.

---

## E. Slice-Status

| Slice | Status |
|---|---|
| 0 Analyse / Vertrag | erledigt (`docs/app-update.md`) |
| 1 Version-UI | erledigt |
| 2 Manueller Check | erledigt |
| 3 Download / Install / Relaunch | erledigt |
| 4 Release-Checkliste | erledigt — [`app-update-release.md`](./app-update-release.md) |

Nächste Arbeit nach Lab-Bedarf: Endpoint `OWNER/…` finalisieren, `createUpdaterArtifacts` für Release-Builds, Alt→Neu-Lab laut Release-Checkliste.

---

## Lab-Verifikations-Checkliste (Kurz)

Ausführlich: [`app-update-release.md`](./app-update-release.md) Abschnitte 7–8.

- [ ] Keypaar offline erzeugt; Private Key nicht im Repo
- [ ] `pubkey` in Updater-Config
- [ ] Altversion installiert
- [ ] Neuversion gebaut mit höherer SemVer
- [ ] Updater-Artefakte + `.sig` erzeugt
- [ ] Manifest zeigt korrekt auf Neu-Artefakt
- [ ] Endpoint erreichbar (HTTPS Prod / dokumentiertes Lab)
- [ ] E2E: check → download → install → relaunch → Version neu
- [ ] Negativ: offline, bad manifest, bad sig mindestens einmal

---

## Betroffene Dateien (Referenz für spätere Slices)

Nur zur Orientierung — in Slice 0 **nicht** ändern:

- `apps/desktop/src/components/SettingsSheet.tsx`
- `apps/desktop/src/components/settings/SettingsParts.tsx`
- `apps/desktop/src/App.css` (nur bei Bedarf)
- Neu: `apps/desktop/src/api/updates.ts`, `apps/desktop/src/hooks/useAppUpdate.ts`
- `apps/desktop/src-tauri/src/lib.rs`, `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`
- `apps/desktop/package.json`

---

## Commit-Grenze Slice 0

- **Erlaubt:** `docs/app-update.md`, optional `docs/README.md`-Indexzeile
- **Verboten:** Settings, Tauri-Plugins, API, Keys, Release-Skripte, Fake-Updater
