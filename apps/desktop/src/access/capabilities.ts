import type { AppAccessSnapshot, Capability } from "./types.ts";
import { isDeveloperEntryVisible } from "./developerAccess.ts";

/** Per-action gates — never “panel open ⇒ allowed”. */
export function hasCapability(
  cap: Capability,
  state: AppAccessSnapshot,
): boolean {
  switch (cap) {
    case "settings:open":
      return true;
    case "backup:create":
      return true;
    case "backup:restore":
    case "admin:reset":
    case "admin:bureau-edit":
      return state.isAdminModeEnabled;
    case "admin:test-unlock":
      // Dev/test-only unlock; available whenever developer mode is on.
      return state.isDeveloperModeEnabled;
    case "developer:open":
      return isDeveloperEntryVisible(state.developerAccessPolicy);
    case "developer:diagnostics":
    case "developer:simulator":
      return state.isDeveloperModeEnabled;
    default: {
      const _exhaustive: never = cap;
      return _exhaustive;
    }
  }
}

export class CapabilityDeniedError extends Error {
  readonly capability: Capability;

  constructor(capability: Capability) {
    super(`Capability denied: ${capability}`);
    this.name = "CapabilityDeniedError";
    this.capability = capability;
  }
}

export function assertCapability(
  cap: Capability,
  state: AppAccessSnapshot,
): void {
  if (!hasCapability(cap, state)) {
    throw new CapabilityDeniedError(cap);
  }
}
