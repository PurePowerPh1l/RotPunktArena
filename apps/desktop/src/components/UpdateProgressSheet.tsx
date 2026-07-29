import { useEffect } from "react";
import { useAppUpdateContext } from "../hooks/AppUpdateProvider";

function progressPercent(
  downloaded: number,
  contentLength: number | null,
): number | null {
  if (!contentLength || contentLength <= 0) return null;
  return Math.min(100, Math.round((downloaded / contentLength) * 100));
}

function sheetTitle(kind: string): string {
  switch (kind) {
    case "downloading":
      return "Update wird geladen";
    case "installing":
      return "Installation startet";
    case "readyToRelaunch":
    case "needsManualRestart":
      return "Update installiert";
    case "error":
      return "Update fehlgeschlagen";
    default:
      return "Neue Version verfügbar";
  }
}

/**
 * Shared update sheet: startup notice + download progress + quiet install handoff.
 *
 * On Windows with installMode quiet the app exits when NSIS starts; the bar
 * covers download only. readyToRelaunch is kept as a non-Windows / fallback path.
 */
export function UpdateProgressSheet() {
  const appUpdate = useAppUpdateContext();
  const {
    status,
    version,
    busy,
    sheetDismissed,
    installSession,
    refreshVersion,
    checkForUpdates,
    beginInstallFromUi,
    dismissUpdateSheet,
    relaunchToApply,
  } = appUpdate;

  useEffect(() => {
    void refreshVersion();
    void checkForUpdates();
  }, [refreshVersion, checkForUpdates]);

  const visible =
    !sheetDismissed &&
    (status.kind === "available" ||
      (installSession &&
        (status.kind === "downloading" ||
          status.kind === "installing" ||
          status.kind === "readyToRelaunch" ||
          status.kind === "needsManualRestart" ||
          status.kind === "error")));

  if (!visible) return null;

  const update =
    status.kind === "available" ||
    status.kind === "downloading" ||
    status.kind === "installing" ||
    status.kind === "readyToRelaunch" ||
    status.kind === "needsManualRestart"
      ? status.update
      : null;

  const percent =
    status.kind === "downloading"
      ? progressPercent(status.progress.downloaded, status.progress.contentLength)
      : status.kind === "installing"
        ? 100
        : null;

  const showBar =
    status.kind === "downloading" || status.kind === "installing";
  const barWidth =
    status.kind === "installing"
      ? 100
      : percent !== null
        ? percent
        : status.kind === "downloading"
          ? 8
          : 0;

  return (
    <div
      className="update-notice-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-notice-title"
    >
      <div className="update-notice-card">
        <p className="update-notice-eyebrow">App-Update</p>
        <h2 id="update-notice-title">{sheetTitle(status.kind)}</h2>

        {update ? (
          <p className="update-notice-versions">
            {version ? (
              <>
                <span className="update-notice-current">{version}</span>
                <span aria-hidden="true"> → </span>
              </>
            ) : null}
            <strong>{update.version}</strong>
          </p>
        ) : null}

        {status.kind === "available" && update?.body ? (
          <p className="update-notice-body">{update.body}</p>
        ) : null}

        {showBar ? (
          <div className="update-notice-progress-block" aria-live="polite">
            <p className="update-notice-progress">
              {status.kind === "downloading"
                ? percent !== null
                  ? `Wird heruntergeladen… ${percent} %`
                  : "Wird heruntergeladen…"
                : "Installation startet… Die App schließt sich und startet neu."}
            </p>
            <div
              className={`update-notice-track${status.kind === "downloading" && percent === null ? " is-indeterminate" : ""}`}
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent ?? undefined}
              aria-label="Update-Fortschritt"
            >
              <div
                className="update-notice-fill"
                style={{ width: `${barWidth}%` }}
              />
            </div>
          </div>
        ) : null}

        {status.kind === "readyToRelaunch" ? (
          <p className="update-notice-body">
            Das Update ist installiert, aber noch nicht aktiv. Jetzt neu starten
            — oder später: Die neue Version läuft ab dem nächsten Start.
          </p>
        ) : null}
        {status.kind === "needsManualRestart" ? (
          <p className="banner-error">{status.message}</p>
        ) : null}
        {status.kind === "error" ? (
          <p className="banner-error">{status.message}</p>
        ) : null}

        <div className="update-notice-actions">
          {status.kind === "available" ? (
            <button
              type="button"
              disabled={busy}
              onClick={() => beginInstallFromUi()}
            >
              Jetzt installieren
            </button>
          ) : null}
          {status.kind === "readyToRelaunch" ? (
            <button
              type="button"
              disabled={busy}
              onClick={() => void relaunchToApply()}
            >
              Jetzt neu starten
            </button>
          ) : null}
          {status.kind === "available" ||
          status.kind === "readyToRelaunch" ||
          status.kind === "needsManualRestart" ||
          status.kind === "error" ? (
            <button
              type="button"
              className="secondary"
              disabled={busy && status.kind !== "error" && status.kind !== "needsManualRestart"}
              onClick={dismissUpdateSheet}
            >
              {status.kind === "available" ? "Später" : "Schließen"}
            </button>
          ) : null}
        </div>

        {status.kind === "available" ? (
          <p className="update-notice-hint">
            Updates lassen sich jederzeit unter Einstellungen → App &amp; Updates
            prüfen und installieren.
          </p>
        ) : null}
      </div>
    </div>
  );
}
