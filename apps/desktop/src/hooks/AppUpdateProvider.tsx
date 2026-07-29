import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useAppUpdate } from "./useAppUpdate";

type AppUpdateApi = ReturnType<typeof useAppUpdate> & {
  /** User dismissed the notice while an update is available (or after error). */
  sheetDismissed: boolean;
  /** True once install was started from notice or Settings — keeps sheet visible. */
  installSession: boolean;
  dismissUpdateSheet: () => void;
  /** Open the shared progress sheet and begin download/install. */
  beginInstallFromUi: () => void;
};

const AppUpdateContext = createContext<AppUpdateApi | null>(null);

export function AppUpdateProvider({ children }: { children: ReactNode }) {
  const update = useAppUpdate();
  const [sheetDismissed, setSheetDismissed] = useState(false);
  const [installSession, setInstallSession] = useState(false);
  const { downloadAndInstall } = update;

  const dismissUpdateSheet = useCallback(() => {
    setSheetDismissed(true);
  }, []);

  const beginInstallFromUi = useCallback(() => {
    setSheetDismissed(false);
    setInstallSession(true);
    void downloadAndInstall();
  }, [downloadAndInstall]);

  const value = useMemo<AppUpdateApi>(
    () => ({
      ...update,
      sheetDismissed,
      installSession,
      dismissUpdateSheet,
      beginInstallFromUi,
    }),
    [update, sheetDismissed, installSession, dismissUpdateSheet, beginInstallFromUi],
  );

  return (
    <AppUpdateContext.Provider value={value}>{children}</AppUpdateContext.Provider>
  );
}

export function useAppUpdateContext(): AppUpdateApi {
  const ctx = useContext(AppUpdateContext);
  if (!ctx) {
    throw new Error("useAppUpdateContext must be used within AppUpdateProvider");
  }
  return ctx;
}
