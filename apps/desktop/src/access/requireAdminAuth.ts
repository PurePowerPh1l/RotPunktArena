import { adminAccessStore } from "./adminAccessStore.ts";

export type AdminAuthMode = "setup" | "unlock";

type OpenHandler = (mode: AdminAuthMode) => void;

let openHandler: OpenHandler | null = null;
let pendingPromise: Promise<boolean> | null = null;
let settlePending: ((ok: boolean) => void) | null = null;

/** Mounted by AdminAuthSheet — single UI owner for setup/unlock. */
export function registerAdminAuthUi(handler: OpenHandler | null): void {
  openHandler = handler;
}

/** Called by the sheet on success / cancel / close. */
export function completeAdminAuth(ok: boolean): void {
  const settle = settlePending;
  settlePending = null;
  pendingPromise = null;
  settle?.(ok);
}

/**
 * Sole gate for sensitive admin actions.
 * Already unlocked → true. Otherwise opens the one auth sheet.
 * Concurrent callers share the same in-flight sheet.
 */
export async function requireAdminAuth(): Promise<boolean> {
  if (adminAccessStore.isAdminModeEnabled) return true;
  if (pendingPromise) return pendingPromise;

  if (!openHandler) {
    console.error("requireAdminAuth: AdminAuthSheet not registered");
    return false;
  }

  const mode: AdminAuthMode =
    adminAccessStore.state === "unconfigured" ? "setup" : "unlock";

  pendingPromise = new Promise<boolean>((resolve) => {
    settlePending = resolve;
    openHandler!(mode);
  });
  return pendingPromise;
}
