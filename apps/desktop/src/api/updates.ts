/**
 * App version / update bridge.
 *
 * Source of truth for the displayed version: Tauri bundle version
 * (`apps/desktop/src-tauri/tauri.conf.json` → `version`), via `getVersion`.
 * Do not read package.json for the Settings UI.
 *
 * Manual check / download / install / relaunch only — no auto-update on start.
 * Stable endpoint + pubkey live in tauri.conf.json. Private signing key
 * stays outside the repo (local `.keys/`, gitignored).
 */
import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";

export type AppUpdateInfo = {
  version: string;
  body?: string | null;
  date?: string | null;
};

export type AppUpdateProgress = {
  downloaded: number;
  contentLength: number | null;
};

/** Current app version from the Tauri bundle (tauri.conf.json). */
export async function getAppVersion(): Promise<string> {
  return getVersion();
}

/**
 * Manual update check against the configured Stable endpoint.
 * Returns null when already up to date. Does not download or install.
 */
export async function checkForAppUpdate(): Promise<AppUpdateInfo | null> {
  const update = await check();
  if (!update) return null;
  try {
    return {
      version: update.version,
      body: update.body ?? null,
      date: update.date ?? null,
    };
  } finally {
    await update.close();
  }
}

/**
 * Re-checks, then downloads and installs the available update.
 * Caller must have confirmed with the user before invoking.
 */
export async function downloadAndInstallAppUpdate(
  onProgress?: (progress: AppUpdateProgress) => void,
): Promise<AppUpdateInfo> {
  const update = await check();
  if (!update) {
    throw new Error("Kein Update mehr verfügbar. Bitte erneut prüfen.");
  }

  let downloaded = 0;
  let contentLength: number | null = null;

  const handleProgress = (event: DownloadEvent) => {
    switch (event.event) {
      case "Started":
        downloaded = 0;
        contentLength = event.data.contentLength ?? null;
        onProgress?.({ downloaded, contentLength });
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        onProgress?.({ downloaded, contentLength });
        break;
      case "Finished":
        onProgress?.({ downloaded, contentLength });
        break;
    }
  };

  try {
    const info: AppUpdateInfo = {
      version: update.version,
      body: update.body ?? null,
      date: update.date ?? null,
    };
    await update.downloadAndInstall(handleProgress);
    return info;
  } finally {
    try {
      await update.close();
    } catch {
      /* resource may already be released after install */
    }
  }
}

/** Restart the app so the installed update can take effect. */
export async function relaunchApp(): Promise<void> {
  await relaunch();
}
