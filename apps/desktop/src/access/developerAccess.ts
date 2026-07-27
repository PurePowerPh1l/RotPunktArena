import type { DeveloperAccessPolicy } from "./types.ts";

/** Dev vs release — Release hides Dev entirely; tauri/vite dev keeps it. */
export function resolveDeveloperAccessPolicy(
  isDev: boolean = Boolean(
    typeof import.meta !== "undefined" &&
      (import.meta as ImportMeta & { env?: { DEV?: boolean } }).env?.DEV,
  ),
): DeveloperAccessPolicy {
  return isDev ? "always-visible" : "disabled";
}

/** Resolved once at module load (Vite injects import.meta.env.DEV). */
export const DEVELOPER_ACCESS_POLICY: DeveloperAccessPolicy =
  resolveDeveloperAccessPolicy();

/** Developer capabilities (diagnostics, mouse-aim gate) — off only when policy is disabled. */
export function isDeveloperModeEnabled(
  policy: DeveloperAccessPolicy = DEVELOPER_ACCESS_POLICY,
): boolean {
  return policy !== "disabled";
}

/** Topbar Dev button — not the same as sheet-open or admin unlock. */
export function isDeveloperEntryVisible(
  policy: DeveloperAccessPolicy = DEVELOPER_ACCESS_POLICY,
): boolean {
  switch (policy) {
    case "always-visible":
      return true;
    case "hidden-trigger":
      // Placeholder: later unlock via discrete gesture; until then hide.
      return false;
    case "disabled":
      return false;
  }
}
