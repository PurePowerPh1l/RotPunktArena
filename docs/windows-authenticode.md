# Windows Authenticode (Code Signing)

**Status:** Vorbereitung ohne Zertifikat — Config/Doku/Verify; echtes Signing erst mit OV-/EV-Zertifikat oder Azure Trusted Signing.  
**Nicht verwechseln:** Tauri-Updater-Signatur (minisign / `.sig` / `pubkey`) ≠ Windows Authenticode.  
**Produktvertrag Updater:** [`app-update.md`](./app-update.md), Release-Ablauf: [`app-update-release.md`](./app-update-release.md).

---

## 1. Ziel

EXE und Installer so signieren, dass Windows einen **bekannten Publisher** zeigt statt „Unknown Publisher“. Das reduziert SmartScreen-Warnungen spürbar — ist aber **keine Garantie**, dass SmartScreen nie warnt (Reputation, Download-Quelle und Zertifikatstyp zählen mit).

Zielartefakte beim Windows-Release:

| Artefakt | Rolle |
|---|---|
| App-EXE im Bundle | Laufzeit-Binary |
| NSIS `*-setup.exe` | Setup-Installer |
| WiX `*.msi` | MSI-Installer (falls Target aktiv) |

---

## 2. Authenticode ≠ Updater-Sig

| | Authenticode | Updater (Tauri) |
|---|---|---|
| Zweck | Publisher / SmartScreen / OS-Vertrauen | Integrität der Update-Artefakte |
| Tool | SignTool / Tauri `bundle.windows` | `tauri signer` / minisign |
| Config | `certificateThumbprint` oder `signCommand` | `plugins.updater.pubkey` + `TAURI_SIGNING_PRIVATE_KEY*` |
| Artefakt | signierte EXE/MSI (Embedded Signature) | `*.sig` + `signature` in `latest.json` |
| Secrets | PFX / Token / Thumbprint | Updater-Private-Key |

Beide können (und sollen) im Release-Build vorkommen. Eins ersetzt das andere nicht.

---

## 3. Build-Reihenfolge (Pflicht)

```text
1. Zertifikat verfügbar (Store oder signCommand)
2. tauri build  →  Authenticode während Bundle (Binary + Installer)
3. createUpdaterArtifacts  →  minisign .sig über die bereits Authenticode-signierten Dateien
4. latest.json / GitHub Release
5. Danach Dateien nicht mehr verändern
```

Nach Authenticode oder Updater-Sig **keine** Nachbearbeitung der EXE/MSI (kein Rebrand, kein Zip-Repack der gleichen Bytes unter anderem Hash). Sonst ist die Updater-Signatur ungültig bzw. Authenticode bricht.

Verifikation Authenticode (lokal):

```powershell
Get-AuthenticodeSignature path\to\RotPunktArena_x.y.z_x64-setup.exe
# Status soll Valid sein (nicht NotSigned)
```

Skript (nach Verify-Slice): [`scripts/verify-windows-authenticode.ps1`](../scripts/verify-windows-authenticode.ps1).

---

## 4. Standardpfad (OV, lokal)

Empfohlen für den manuellen Release-Pfad dieses Repos (kein CI in diesem Slice):

1. Code-Signing-Zertifikat kaufen (OV; **kein** SSL-/Webserver-Zertifikat).
2. Als `.pfx` exportieren / importieren in `Cert:\CurrentUser\My`.
3. Thumbprint notieren (`certmgr.msc` → Details).
4. Beispiel-Config kopieren:

```powershell
Copy-Item apps/desktop/src-tauri/tauri.windows-signing.conf.example.json `
  apps/desktop/src-tauri/tauri.windows-signing.conf.json
# Thumbprint eintragen — Datei ist gitignored
```

5. Release-Build mit Config-Merge (Tauri CLI `--config`), plus Updater-Key-Env wie in der Release-Checkliste.
6. Authenticode prüfen, dann Manifest/Release wie bisher.

Felder laut Tauri:

- `bundle.windows.certificateThumbprint`
- `bundle.windows.digestAlgorithm` → typisch `sha256`
- `bundle.windows.timestampUrl` → CA-Timestamp (z. B. DigiCert)

**Nicht** in die committed `tauri.conf.json` schreiben — sonst scheitern unsigned Alltag-Builds ohne Zertifikat.

---

## 5. Alternativen (später)

| Pfad | Wann | Hook |
|---|---|---|
| EV-Zertifikat (oft Token/HSM) | Strengere Org-Validierung, oft bessere SmartScreen-Reputation | meist `signCommand` laut CA-Doku |
| Azure Trusted Signing / Artifact Signing | Cloud, kein lokales PFX | `bundle.windows.signCommand` + Azure-ENV |

Nicht in diesem Prep-Branch verdrahten — nur als Folgeweg notiert.

---

## 6. ENV- / Secret-Namen (Platzhalter)

Keine Werte committen. Lokal oder Secret-Store:

| Name | Bedeutung |
|---|---|
| `WINDOWS_CERTIFICATE_THUMBPRINT` | SHA1-Thumbprint des Code-Signing-Zerts im Store (Dokumentation / lokales Mergen in Override-Config) |
| `WINDOWS_PFX_PATH` | Pfad zur `.pfx` (Import; nie ins Repo) |
| `WINDOWS_PFX_PASSWORD` | Export-Passwort der `.pfx` |
| `WINDOWS_TIMESTAMP_URL` | Timestamp-Server der CA |

Updater-Secrets bleiben getrennt: `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PATH` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

---

## 7. SmartScreen — realistische Erwartung

- **Mit gültigem Authenticode:** Publisher sichtbar; Warnungen deutlich seltener / milder als bei unsigned.
- **Ohne Reputation:** Neue Publisher können trotzdem kurz SmartScreen sehen — Downloads und Zeit helfen.
- **EV / etablierte CA / Store:** oft schneller Vertrauen, aber teurer/komplexer.
- **Nur Updater-`.sig`:** ändert SmartScreen **nicht**.

---

## 8. Explizit nicht hier

- Zertifikatkauf oder Import automatisieren
- CI/CD für Signing
- Thumbprint in Prod-`tauri.conf.json`
- Lab→Prod-Rotation des **Updater**-Keypaars (eigenes Thema; siehe Release-Doku)
