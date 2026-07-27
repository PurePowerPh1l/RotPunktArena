import { useCallback, useRef, useState } from "react";
import * as updatesApi from "../api/updates";
import type { AppUpdateInfo, AppUpdateProgress } from "../api/updates";

export type AppUpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "upToDate" }
  | { kind: "available"; update: AppUpdateInfo }
  | { kind: "downloading"; update: AppUpdateInfo; progress: AppUpdateProgress }
  | { kind: "installing"; update: AppUpdateInfo }
  | { kind: "readyToRelaunch"; update: AppUpdateInfo }
  | { kind: "needsManualRestart"; update: AppUpdateInfo; message: string }
  | { kind: "error"; message: string }
  | { kind: "devOnly" };

/**
 * Manual update state machine (check → download/install → relaunch).
 * No auto-check on mount — caller must invoke actions explicitly.
 */
export function useAppUpdate() {
  const [version, setVersion] = useState<string | null>(null);
  const [status, setStatus] = useState<AppUpdateStatus>({ kind: "idle" });
  const inflight = useRef(false);
  /** Target version remembered across download → relaunch. */
  const pendingUpdate = useRef<AppUpdateInfo | null>(null);

  const refreshVersion = useCallback(async () => {
    try {
      setVersion(await updatesApi.getAppVersion());
    } catch (e) {
      setStatus({ kind: "error", message: String(e) });
    }
  }, []);

  const checkForUpdates = useCallback(async () => {
    if (inflight.current) return;
    if (import.meta.env.DEV) {
      setStatus({ kind: "devOnly" });
      return;
    }

    inflight.current = true;
    setStatus({ kind: "checking" });
    try {
      const update = await updatesApi.checkForAppUpdate();
      if (!update) {
        pendingUpdate.current = null;
        setStatus({ kind: "upToDate" });
      } else {
        pendingUpdate.current = update;
        setStatus({ kind: "available", update });
      }
    } catch (e) {
      setStatus({ kind: "error", message: String(e) });
    } finally {
      inflight.current = false;
    }
  }, []);

  /**
   * Download + install after the UI confirmed with the user.
   * Ends in readyToRelaunch — never claims the new version is already running.
   */
  const downloadAndInstall = useCallback(async () => {
    if (inflight.current) return;
    if (import.meta.env.DEV) {
      setStatus({ kind: "devOnly" });
      return;
    }

    const known = pendingUpdate.current;
    if (!known) {
      setStatus({
        kind: "error",
        message: "Kein Update ausgewählt. Bitte zuerst prüfen.",
      });
      return;
    }

    inflight.current = true;
    setStatus({
      kind: "downloading",
      update: known,
      progress: { downloaded: 0, contentLength: null },
    });

    try {
      const installed = await updatesApi.downloadAndInstallAppUpdate(
        (progress) => {
          setStatus({
            kind: "downloading",
            update: known,
            progress,
          });
        },
      );
      pendingUpdate.current = installed;
      setStatus({ kind: "installing", update: installed });
      setStatus({ kind: "readyToRelaunch", update: installed });
    } catch (e) {
      setStatus({ kind: "error", message: String(e) });
    } finally {
      inflight.current = false;
    }
  }, []);

  const relaunchToApply = useCallback(async () => {
    if (inflight.current) return;
    const update = pendingUpdate.current;
    if (!update) {
      setStatus({
        kind: "error",
        message: "Kein installiertes Update zum Neustart vorhanden.",
      });
      return;
    }

    inflight.current = true;
    try {
      await updatesApi.relaunchApp();
      // If relaunch returns without exiting, fall through to manual hint.
      setStatus({
        kind: "needsManualRestart",
        update,
        message:
          "Neustart konnte nicht automatisch ausgeführt werden. Bitte die App manuell schließen und erneut öffnen.",
      });
    } catch (e) {
      setStatus({
        kind: "needsManualRestart",
        update,
        message: `Automatischer Neustart fehlgeschlagen: ${String(e)}. Bitte die App manuell neu starten.`,
      });
    } finally {
      inflight.current = false;
    }
  }, []);

  const busy =
    status.kind === "checking" ||
    status.kind === "downloading" ||
    status.kind === "installing";

  return {
    version,
    status,
    refreshVersion,
    checkForUpdates,
    downloadAndInstall,
    relaunchToApply,
    checking: status.kind === "checking",
    busy,
  };
}
