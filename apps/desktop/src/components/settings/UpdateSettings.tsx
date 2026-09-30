import type { AppUpdateStatus } from "../../hooks/useAppUpdate";
import { useAppUpdateContext } from "../../hooks/AppUpdateProvider";
import { confirmDialog } from "../../hooks/useAppDialog";
import { SettingsHint, SettingsInfoRow, SettingsSection, type SettingsStatusTone } from "./SettingsParts";

function updateStatusLabel(status: AppUpdateStatus): string {
  switch (status.kind) {
    case "idle":
      return "Noch nicht geprüft";
    case "checking":
      return "Suche…";
    case "upToDate":
      return "Aktuell";
    case "available":
      return `Update ${status.update.version} verfügbar`;
    case "downloading": {
      const { downloaded, contentLength } = status.progress;
      if (contentLength && contentLength > 0) {
        const pct = Math.min(100, Math.round((downloaded / contentLength) * 100));
        return `Lade herunter… ${pct}%`;
      }
      return "Lade herunter…";
    }
    case "installing":
      return "Installiere…";
    case "readyToRelaunch":
      return `Installiert — Neustart nötig (${status.update.version})`;
    case "needsManualRestart":
      return `Neustart manuell nötig (${status.update.version})`;
    case "error":
      return "Fehler";
    case "devOnly":
      return "Nur in installierter App";
  }
}

function updateStatusTone(status: AppUpdateStatus): SettingsStatusTone {
  switch (status.kind) {
    case "idle":
      return "neutral";
    case "checking":
    case "downloading":
    case "installing":
      return "progress";
    case "upToDate":
      return "ok";
    case "available":
      return "idle";
    case "readyToRelaunch":
    case "needsManualRestart":
      return "idle";
    case "error":
      return "idle";
    case "devOnly":
      return "neutral";
  }
}

export function UpdateSettings({ open, onOpenChange, appUpdate }: {
  open: boolean;
  onOpenChange: () => void;
  appUpdate: ReturnType<typeof useAppUpdateContext>;
}) {
  return (
      <SettingsSection
        title="App & Updates"
        description="Version, Prüfung und Installation von Updates."
        open={open}
        onOpenChange={onOpenChange}
      >
        <div className="settings-info-block">
          <SettingsInfoRow
            label="Aktuelle Version"
            value={appUpdate.version ?? "…"}
          />
          <SettingsInfoRow
            label="Update-Status"
            value={updateStatusLabel(appUpdate.status)}
            statusTone={updateStatusTone(appUpdate.status)}
          />
        </div>
        {appUpdate.status.kind === "error" ? (
          <p className="banner-error">{appUpdate.status.message}</p>
        ) : null}
        {appUpdate.status.kind === "needsManualRestart" ? (
          <p className="banner-error">{appUpdate.status.message}</p>
        ) : null}
        {appUpdate.status.kind === "devOnly" ? (
          <SettingsHint>
            Updates sind in der Entwicklungsansicht nicht verfügbar. Bitte eine
            installierte App-Version verwenden.
          </SettingsHint>
        ) : appUpdate.status.kind === "available" ? (
          <SettingsHint>
            Version {appUpdate.status.update.version} ist verfügbar. Download
            und Installation starten nur nach Ihrer Bestätigung.
          </SettingsHint>
        ) : appUpdate.status.kind === "readyToRelaunch" ? (
          <SettingsHint>
            Update {appUpdate.status.update.version} ist installiert, aber noch
            nicht aktiv. Bitte neu starten — die laufende Sitzung zeigt weiter
            die alte Version.
          </SettingsHint>
        ) : appUpdate.status.kind === "downloading" ||
          appUpdate.status.kind === "installing" ? (
          <SettingsHint>
            Update läuft — Fortschritt im Update-Fenster. Unter Windows schließt
            sich die App nach dem Download und startet neu.
          </SettingsHint>
        ) : appUpdate.status.kind === "needsManualRestart" ? (
          <SettingsHint>
            Das Update ist installiert. Bitte die App manuell schließen und
            erneut öffnen.
          </SettingsHint>
        ) : (
          <SettingsHint>
            Prüfung nur manuell. Download und Installation nach Bestätigung —
            Fortschritt erscheint in einem eigenen Fenster.
          </SettingsHint>
        )}
        <div className="settings-connection-actions">
          {appUpdate.status.kind === "available" ? (
            <button
              type="button"
              className="settings-action-primary"
              disabled={appUpdate.busy}
              onClick={() => {
                if (appUpdate.status.kind !== "available") return;
                const ver = appUpdate.status.update.version;
                void (async () => {
                  const ok = await confirmDialog({
                    title: "Update installieren?",
                    body: `Update ${ver} herunterladen und installieren?\n\nDie App schließt sich danach und startet neu.`,
                    confirmLabel: "Installieren",
                    eyebrow: "App-Update",
                  });
                  if (!ok) return;
                  appUpdate.beginInstallFromUi();
                })();
              }}
            >
              Update installieren
            </button>
          ) : null}
          <button
            type="button"
            className={
              appUpdate.status.kind === "available"
                ? "secondary settings-action-secondary"
                : "settings-action-primary"
            }
            disabled={appUpdate.busy}
            onClick={() => void appUpdate.checkForUpdates()}
          >
            {appUpdate.checking ? "Suche…" : "Nach Updates suchen"}
          </button>
          {appUpdate.status.kind === "readyToRelaunch" ||
          appUpdate.status.kind === "needsManualRestart" ? (
            <button
              type="button"
              className="settings-action-primary"
              disabled={appUpdate.busy}
              onClick={() => void appUpdate.relaunchToApply()}
            >
              Jetzt neu starten
            </button>
          ) : null}
        </div>
      </SettingsSection>
  );
}
