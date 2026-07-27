import { adminAccessStore } from "./adminAccessStore.ts";
import {
  DEVELOPER_ACCESS_POLICY,
  isDeveloperModeEnabled,
} from "./developerAccess.ts";
import type { AppAccessSnapshot } from "./types.ts";

/** Current access snapshot for capability checks (UI + handlers). */
export function getAppAccessSnapshot(): AppAccessSnapshot {
  const adminAccessState = adminAccessStore.state;
  return {
    adminAccessState,
    isAdminModeEnabled: adminAccessState === "unlocked",
    isDeveloperModeEnabled: isDeveloperModeEnabled(DEVELOPER_ACCESS_POLICY),
    developerAccessPolicy: DEVELOPER_ACCESS_POLICY,
  };
}
