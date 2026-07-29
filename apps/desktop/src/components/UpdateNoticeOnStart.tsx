import { useEffect, useState } from "react";
import { useAppUpdate } from "../hooks/useAppUpdate";

function progressPercent(downloaded: number, contentLength: number | null): number | null {
  if (!contentLength || contentLength <= 0) return null;
  return Math.min(100, Math.round((downloaded / contentLength) * 100));
}

/**
 * One-shot update notice at app start.
 *
 * Checks the Stable endpoint once on mount and shows a modal only when a
 * newer version exists. Silent on check errors / dev builds / up-to-date —
 * the manual check in Settings stays the fallback. Install runs only after
 * explicit confirmation (product contract: no auto-update).
 */
export function UpdateNoticeOnStart() {
  const appUpdate = useAppUpdate();
  const [dismissed, setDismissed] = useState(false);
  /** True once the user pressed install — from then on errors stay visible. */
  const [installStarted, setInstallStarted] = useState(false);

  const { refreshVersion, checkForUpdates } = appUpdate;
  useEffect(() => {
    void refreshVersion();
    void checkForUpdates();
  }, [refreshVersion, checkForUpdates]);

  const { status } = appUpdate;
  const visible =
    !dismissed &&
    (status.kind === "available" ||
      (installStarted &&
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
      : null;

  return (
    <div
      className="update-notice-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-notice-title"
    >
      <div className="update-notice-card">
        <p className="update-notice-eyebrow">App-Update</p>
        <h2 id="update-notice-title">
          {status.kind === "readyToRelaunch" || status.kind === "needsManualRestart"
            ? "Update installiert"
            : "Neue Version verfügbar"}
        </h2>

        {update ? (
          <p className="update-notice-versions">
            {appUpdate.version ? (
              <>
                <span className="update-notice-current">{appUpdate.version}</span>
                <span aria-hidden="true"> → </span>
              </>
            ) : null}
            <strong>{update.version}</strong>
          </p>
        ) : null}

        {status.kind === "available" && update?.body ? (
          <p className="update-notice-body">{update.body}</p>
        ) : null}

        {status.kind === "downloading" ? (
          <p className="update-notice-progress" aria-live="polite">
            Wird heruntergeladen…{percent !== null ? ` ${percent} %` : ""}
          </p>
        ) : null}
        {status.kind === "installing" ? (
          <p className="update-notice-progress" aria-live="polite">
            Wird installiert…
          </p>
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
              disabled={appUpdate.busy}
              onClick={() => {
                setInstallStarted(true);
                void appUpdate.downloadAndInstall();
              }}
            >
              Jetzt installieren
            </button>
          ) : null}
          {status.kind === "readyToRelaunch" ? (
            <button
              type="button"
              disabled={appUpdate.busy}
              onClick={() => void appUpdate.relaunchToApply()}
            >
              Jetzt neu starten
            </button>
          ) : null}
          <button
            type="button"
            className="secondary"
            disabled={appUpdate.busy}
            onClick={() => setDismissed(true)}
          >
            Später
          </button>
        </div>

        <p className="update-notice-hint">
          Updates lassen sich jederzeit unter Einstellungen → App &amp; Updates
          prüfen und installieren.
        </p>
      </div>
    </div>
  );
}
