import { useCallback, useMemo, useSyncExternalStore } from "react";
import {
  adminAccessStore,
  DEVELOPER_ACCESS_POLICY,
  getAppAccessSnapshot,
  hasCapability,
  isDeveloperEntryVisible,
  isDeveloperModeEnabled,
  type AppAccessSnapshot,
  type Capability,
} from "../access";

/**
 * Subscribes to ephemeral admin unlock + reads developer policy.
 * Sheet-open flags stay in the caller (pure UI).
 */
export function useAppAccess() {
  const adminAccessState = useSyncExternalStore(
    (onStoreChange) => adminAccessStore.subscribe(onStoreChange),
    adminAccessStore.getSnapshot,
    adminAccessStore.getSnapshot,
  );

  const snapshot: AppAccessSnapshot = useMemo(
    () => ({
      adminAccessState,
      isAdminModeEnabled: adminAccessState === "unlocked",
      isDeveloperModeEnabled: isDeveloperModeEnabled(DEVELOPER_ACCESS_POLICY),
      developerAccessPolicy: DEVELOPER_ACCESS_POLICY,
    }),
    [adminAccessState],
  );

  const can = useCallback(
    (cap: Capability) => hasCapability(cap, snapshot),
    [snapshot],
  );

  const setAdminUnlockedForTests = useCallback((on: boolean) => {
    if (on) {
      if (!hasCapability("admin:test-unlock", getAppAccessSnapshot())) return;
      adminAccessStore.enableAdminForTests();
    } else {
      adminAccessStore.lock();
    }
  }, []);

  return {
    ...snapshot,
    can,
    setAdminUnlockedForTests,
    isDeveloperEntryVisible: isDeveloperEntryVisible(
      snapshot.developerAccessPolicy,
    ),
  };
}
