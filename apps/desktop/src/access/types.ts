/**
 * Access / capability model for Settings + Developer sheets.
 * Sheet-open UI state is separate — never derive permissions from an open panel.
 */

/** Admin auth readiness — ephemeral unlock only until real setup exists. */
export type AdminAccessState = "unconfigured" | "locked" | "unlocked";

/**
 * Central policy for Dev entry visibility.
 * Current product stage: always-visible. Later: hidden-trigger or disabled.
 */
export type DeveloperAccessPolicy =
  | "always-visible"
  | "hidden-trigger"
  | "disabled";

export type Capability =
  | "settings:open"
  | "backup:create"
  | "backup:restore"
  | "admin:reset"
  | "admin:bureau-edit"
  | "admin:test-unlock"
  | "developer:open"
  | "developer:diagnostics"
  | "developer:simulator";

export type AppAccessSnapshot = {
  adminAccessState: AdminAccessState;
  isAdminModeEnabled: boolean;
  isDeveloperModeEnabled: boolean;
  developerAccessPolicy: DeveloperAccessPolicy;
};
