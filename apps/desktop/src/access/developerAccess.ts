import type { DeveloperAccessPolicy } from "./types.ts";

/** Current stage: Dev entry always shown. Flip here later to hide/disable. */
export const DEVELOPER_ACCESS_POLICY: DeveloperAccessPolicy = "always-visible";

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
