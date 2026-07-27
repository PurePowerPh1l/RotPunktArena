/**
 * Transport/recovery for ui.prefs — sole FE caller of get/set_ui_prefs.
 * No localStorage; optimistic merge + serialized saves with rollback on error.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { UiPrefs } from "@rotpunktarena/domain";
import { UI_PREFS_LOAD_PLACEHOLDER } from "@rotpunktarena/domain";
import * as settingsApi from "../api/settings";
import { mergeUiPrefsPatch } from "../lib/uiPrefsLogic";
import {
  emptySaveQueue,
  onSaveFailed,
  onSaveFinished,
  onSaveRequested,
  type SaveQueueState,
} from "../lib/prefsSaveQueue";

export type UiPrefsStatus = "loading" | "ready" | "saving" | "error";

export type UiPrefsState = {
  prefs: UiPrefs;
  status: UiPrefsStatus;
  error: string | null;
  updatePrefs: (patch: Partial<UiPrefs>) => void;
  retryLoad: () => Promise<void>;
};

export { resolveStartView } from "../lib/uiPrefsLogic";

export function useUiPrefs(): UiPrefsState {
  const [prefs, setPrefs] = useState<UiPrefs>(UI_PREFS_LOAD_PLACEHOLDER);
  const [status, setStatus] = useState<UiPrefsStatus>("loading");
  const [error, setError] = useState<string | null>(null);

  const confirmedRef = useRef<UiPrefs>(UI_PREFS_LOAD_PLACEHOLDER);
  const queueRef = useRef<SaveQueueState<UiPrefs>>(emptySaveQueue());
  const mountedRef = useRef(true);

  const applyLoaded = useCallback((loaded: UiPrefs) => {
    confirmedRef.current = loaded;
    setPrefs(loaded);
    setError(null);
    setStatus("ready");
  }, []);

  const load = useCallback(async () => {
    setStatus("loading");
    setError(null);
    try {
      const loaded = await settingsApi.getUiPrefs();
      if (!mountedRef.current) return;
      applyLoaded(loaded);
    } catch (e) {
      if (!mountedRef.current) return;
      setError(String(e));
      setStatus("error");
    }
  }, [applyLoaded]);

  useEffect(() => {
    mountedRef.current = true;
    void load();
    return () => {
      mountedRef.current = false;
    };
  }, [load]);

  const flushSave = useCallback(async (next: UiPrefs) => {
    const enqueued = onSaveRequested(queueRef.current, next);
    queueRef.current = enqueued.state;
    if (!enqueued.start) return;

    setStatus("saving");
    setError(null);
    let toSave: UiPrefs | null = enqueued.start;
    try {
      while (toSave) {
        const saved = await settingsApi.setUiPrefs(toSave);
        if (!mountedRef.current) return;
        const finished = onSaveFinished(queueRef.current);
        queueRef.current = finished.state;
        if (finished.continueWith) {
          toSave = finished.continueWith;
          continue;
        }
        confirmedRef.current = saved;
        setPrefs(saved);
        setStatus("ready");
        toSave = null;
      }
    } catch (e) {
      if (!mountedRef.current) return;
      queueRef.current = onSaveFailed();
      setPrefs(confirmedRef.current);
      setError(String(e));
      setStatus("error");
    }
  }, []);

  const updatePrefs = useCallback(
    (patch: Partial<UiPrefs>) => {
      if (status === "loading") return;
      setPrefs((prev) => {
        const next = mergeUiPrefsPatch(prev, patch);
        void flushSave(next);
        return next;
      });
    },
    [flushSave, status],
  );

  return {
    prefs,
    status,
    error,
    updatePrefs,
    retryLoad: load,
  };
}
