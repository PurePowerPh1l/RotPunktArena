# App-Update — Release-Checkliste (Stable)

**Status:** Slice 4 — manuelle Release-Hygiene, keine CI-Automatisierung.  
**Produktvertrag:** siehe [`app-update.md`](./app-update.md) (kein Auto-Update, nur Stable, öffentliche GitHub Releases).  
**Secrets:** Private Keys und Passwörter **nie** ins Repo, nie in Commits, nie in Docs kopieren.

---

## 1. Voraussetzungen

| Item | Soll |
|---|---|
| Updater-Keypaar | Offline erzeugt (`tauri signer generate`); Private Key nur lokal / Secret Store |
| Public Key | In `apps/desktop/src-tauri/tauri.conf.json` → `plugins.updater.pubkey` |
| Endpoint | `plugins.updater.endpoints` zeigt auf **öffentliches** `latest.json` (HTTPS) |
| Repo | Öffentliche GitHub Releases (nicht privat — Auth/ACL-Hürden vermeiden) |
| Lab | Update-E2E nur mit **installierter** Altversion testen, nicht mit `tauri dev` |

Lokal (Stand Slice 2+): Keypaar unter `.keys/` (gitignored). Prod-Key sollte ein separates, gesichertes Paar sein — Lab-Key nicht stillschweigend als Prod übernehmen, ohne das bewusst zu entscheiden.

---

## 2. Version-Bump-Regel (eine Quelle)

**Maßgeblich:** `apps/desktop/src-tauri/tauri.conf.json` → `"version"`.

Bei jedem Release **dieselbe** SemVer setzen in:

1. `apps/desktop/src-tauri/tauri.conf.json` → `version` (**Quelle**)
2. `apps/desktop/src-tauri/Cargo.toml` → `version` (Diagnose / `CARGO_PKG_VERSION`)
3. `apps/desktop/package.json` → `version`
4. Root-`package.json` → `version` (Workspace-Konsistenz)

UI und Updater lesen die Tauri-Bundle-Version (`getVersion` / Updater-Vergleich).  
Kein paralleles „Version aus package.json anzeigen“.

---

## 3. Updater-Artefakte aktivieren

Vor dem **Release-Build** in `tauri.conf.json` unter `bundle`:

```json
"createUpdaterArtifacts": true
```

Ohne dieses Flag entstehen keine `.sig`-Dateien — Manifest wäre unbrauchbar.

Hinweis: Unsigned Alltag-Builds ohne Signing-Env können mit diesem Flag scheitern. Für Dev/CI ohne Key entweder Flag aus oder Signing-Env setzen. Prod-Release: Flag an + Key gesetzt.

---

## 4. Signierter Release-Build

PowerShell (Windows), Private Key **nicht** committen:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PATH = "D:\path\to\reddot-updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<password if any>"
npm run desktop:build
```

Alternativ Key-Inhalt in `TAURI_SIGNING_PRIVATE_KEY` (statt Pfad).

Erwartete Artefakte (u. a.):

- Installer unter `apps/desktop/src-tauri/target/release/bundle/` (NSIS/MSI je nach Target)
- Signaturen: z. B. `*-setup.exe.sig`, `*.msi.sig`

`.sig`-Datei = **Inhalt** (Base64-Text) für das Manifest — kein Dateipfad, keine URL im `signature`-Feld.

---

## 4b. Windows-Install-Modus (quiet)

In `tauri.conf.json` unter `plugins.updater`:

```json
"windows": { "installMode": "quiet" }
```

- `quiet` → NSIS `/S /R`: kein Windows-Installer-Fenster; App beendet sich beim Install-Start und wird neu gestartet.
- Default ohne Eintrag wäre `passive` (`/P /R`) mit sichtbarer Installer-UI — nicht gewünscht.
- In-App-Progressbar deckt nur den **Download** ab; Install selbst läuft nach App-Exit im stillen Setup.

---

## 5. Manifest `latest.json`

Statisches JSON für öffentliche GitHub Releases (Tauri Static JSON):

```json
{
  "version": "0.2.0",
  "notes": "Kurzbeschreibung für Nutzer (optional)",
  "pub_date": "2026-07-27T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<INHALT der .sig-Datei>",
      "url": "https://github.com/OWNER/reddot-arena/releases/download/v0.2.0/RedDot.Arena_0.2.0_x64-setup.exe"
    }
  }
}
```

Pflicht: `version`, `platforms.<target>.url`, `platforms.<target>.signature`.  
`OWNER/reddot-arena` durch das echte öffentliche Repo ersetzen; Dateinamen an echte Bundle-Namen anpassen.

Endpoint in `tauri.conf.json` (Beispiel):

```text
https://github.com/OWNER/reddot-arena/releases/latest/download/latest.json
```

`latest`-Tag/Asset so pflegen, dass `…/releases/latest/download/latest.json` die aktuelle Stable-Manifest-Datei liefert (Asset am neuesten Release oder konsistente Weiterleitung).

---

## 6. Öffentliches GitHub Release (manuell)

1. Version bumpen (Abschnitt 2), committen, taggen z. B. `v0.2.0`.
2. Signierten Build erzeugen (Abschnitte 3–4).
3. GitHub Release **öffentlich** anlegen (`gh release create` oder UI).
4. Hochladen: Installer + `latest.json` (und ggf. `.sig` als Zusatzartefakt zur Nachvollziehbarkeit — für den Updater zählt der Sig-**Inhalt** im JSON).
5. Endpoint-URL in `tauri.conf.json` prüfen (OWNER/REPO final, nicht Platzhalter).
6. Kein Private Release als Default.

CI später: derselbe Ablauf als Workflow — **nicht** Teil dieses Slices.

---

## 7. Lab-Verifikation Alt → Neu

Vor Produktiv-Vertrauen einmal durchspielen:

1. **Altversion** installieren (z. B. `0.1.0`), App starten, Version in Einstellungen → App & Updates notieren.
2. Neuversion bauen/signieren (`0.2.0`), Manifest zeigt auf Neu-Installer + korrekte Signature.
3. Manifest unter dem Endpoint erreichbar (Prod: HTTPS; Lab-HTTP nur mit explizit dokumentierter unsicherer Dev-Config — nicht in Prod-Config belassen).
4. In der **installierten** Alt-App: „Nach Updates suchen“ → verfügbar.
5. Confirm → In-App-Progress-Sheet (Download-Balken) → stille NSIS-Installation (`installMode: quiet`); App schließt sich und startet neu.
6. Nach Start: neue Version sichtbar (kein separates „Jetzt neu starten“ nötig unter Windows mit quiet+`/R`).
7. Negativ mindestens einmal: offline / kaputtes Manifest / falsche Sig.

Meta notieren: Commit, Dirty-Tree, OS — Baseline nicht mit Patches überschreiben.

---

## 8. Kurz-Checkliste (Release-Tag)

- [ ] `plugins.updater.windows.installMode` = `quiet` (kein Passive-Installer-Fenster)
- [ ] SemVer in `tauri.conf.json` + Cargo.toml + beiden package.json synchron
- [ ] `createUpdaterArtifacts: true` für diesen Build
- [ ] `TAURI_SIGNING_PRIVATE_KEY*` gesetzt; Key nicht im Repo
- [ ] `npm run desktop:build` erfolgreich; `.sig` vorhanden
- [ ] `latest.json`: Version, URL, **Sig-Inhalt** korrekt
- [ ] Öffentliches GitHub Release inkl. Installer + `latest.json`
- [ ] Endpoint in Config zeigt auf öffentliches Stable-Manifest
- [ ] Lab Alt→Neu grün (Abschnitt 7)
- [ ] Keine Secrets in Commit/PR/CI-Logs

---

## 9. Explizit nicht in diesem Slice

- Vollautomatisierte CI/CD-Pipeline
- Beta/Canary-Kanäle
- Private Releases als Standard
- Authenticode / SmartScreen (getrennt von Updater-Sig)
- Fake-Updater ohne echte Signatur
- Secrets oder Private Keys im Repository
