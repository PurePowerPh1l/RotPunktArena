import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LiveState } from "@rotpunktarena/domain";
import { trainingSaveUiMessage } from "@rotpunktarena/domain";
import * as api from "../api/commands";
import { chooseLiveSnapshot } from "../lib/liveRevision.ts";
import { useAsyncAction } from "./useAsyncAction";

function isRunning(status: LiveState["status"]): boolean {
  return status === "searching" || status === "connected";
}

function detailAfterStop(s: LiveState): string | null {
  const saveMsg = s.trainingSave ? trainingSaveUiMessage(s.trainingSave) : null;
  if (saveMsg) return saveMsg;
  if (s.shotCount > 0 && (s.session?.competitionId || s.session?.entryId)) {
    return `Session beendet (${s.shotCount} Schüsse) — unter Verwaltung → Ergebnisse`;
  }
  return null;
}

function detailAfterReset(s: LiveState): string {
  const save = s.trainingSave;
  if (save?.reason === "saved") {
    return `Neue Serie — vorherige gespeichert (${save.shotCount} Schüsse)`;
  }
  if (save?.reason === "too_short") {
    return `Neue Serie — vorherige zu kurz (${save.shotCount}/${save.minShots}), nicht in Statistik`;
  }
  if (save?.reason === "endless") {
    return `Neue Serie — Endlosmodus (${save.shotCount} Schüsse, nicht gespeichert)`;
  }
  return "Neue Serie gestartet";
}

let epochCounter = 0;

export function useLiveSession() {
  const [state, setState] = useState<LiveState | null>(null);
  const [detail, setDetail] = useState<string | null>(null);
  const { busy, run: runExclusive } = useAsyncAction();
  const activeEpoch = useRef(0);
  const current = useRef<LiveState | null>(null);
  const acceptState = useCallback((snapshot: LiveState) => {
    if (activeEpoch.current === 0) return false;
    const selected = chooseLiveSnapshot(current.current, snapshot);
    if (selected === current.current) return false;
    current.current = selected;
    setState(selected);
    return true;
  }, []);

  const refresh = useCallback(async () => {
    const epoch = activeEpoch.current;
    const snapshot = await api.getLiveState();
    if (epoch !== 0 && epoch === activeEpoch.current) acceptState(snapshot);
  }, [acceptState]);

  useEffect(() => {
    const epoch = ++epochCounter;
    activeEpoch.current = epoch;
    let unlisten: (() => void) | undefined;
    let resync: ReturnType<typeof setInterval> | undefined;
    const active = () => activeEpoch.current === epoch;
    // Subscribe before the initial snapshot. Each event is a complete authoritative
    // projection, so missing/reordered events are repaired without applying deltas.
    void (async () => {
      try {
        const stop = await listen<{ state: LiveState; detail: string | null }>("live_state", (event) => {
          if (active() && acceptState(event.payload.state)) {
            const save = event.payload.state.seriesComplete && event.payload.state.trainingSave;
            setDetail(save ? trainingSaveUiMessage(save) ?? event.payload.detail : event.payload.detail);
          }
        });
        if (!active()) { stop(); return; }
        unlisten = stop;
        const snapshot = await api.getLiveState();
        if (active()) acceptState(snapshot);
        if (active()) resync = setInterval(() => {
          if (active() && document.visibilityState === "visible") {
            void refresh().catch((error) => { if (active()) setDetail(String(error)); });
          }
        }, 5000);
      } catch (error) {
        if (active()) setDetail(String(error));
      }
    })();
    return () => {
      if (active()) activeEpoch.current = 0;
      unlisten?.();
      if (resync) clearInterval(resync);
    };
  }, [acceptState, refresh]);

  const startTraining = useCallback(
    async (
      shooterName: string,
      useSimulator: boolean,
      personId?: string | null,
      endless?: boolean,
    ) => {
      await runExclusive(async () => {
        try {
          const s = await api.startTraining(shooterName, useSimulator, personId, endless);
          if (!acceptState(s)) return;
          setDetail(endless ? "Endlosmodus — Schüsse werden nicht gespeichert" : null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive, acceptState],
  );

  const startEntry = useCallback(
    async (entryId: string, useSimulator: boolean) => {
      await runExclusive(async () => {
        try {
          const s = await api.startEntrySession(entryId, useSimulator);
          if (!acceptState(s)) return;
          setDetail(null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive, acceptState],
  );

  /** Optional prep before Entry-Start in einer Busy-Hülle. */
  const startEntryPrepared = useCallback(
    async (
      entryId: string,
      useSimulator: boolean,
      prep?: () => Promise<void>,
    ) => {
      await runExclusive(async () => {
        try {
          if (prep) await prep();
          const s = await api.startEntrySession(entryId, useSimulator);
          if (!acceptState(s)) return;
          setDetail(null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive, acceptState],
  );

  /** Stop (falls laufend) + nächsten Entry starten — atomar gegen Doppelklick. */
  const stopThenStartEntry = useCallback(
    async (nextEntryId: string, useSimulator: boolean) => {
      await runExclusive(async () => {
        try {
          const current = await api.getLiveState();
          if (isRunning(current.status)) {
            const ended = await api.endTraining();
            if (acceptState(ended)) setDetail(detailAfterStop(ended));
          }
          const s = await api.startEntrySession(nextEntryId, useSimulator);
          if (!acceptState(s)) return;
          setDetail(null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive, acceptState],
  );

  const stop = useCallback(async () => {
    await runExclusive(async () => {
      try {
        const s = await api.endTraining();
        if (!acceptState(s)) return;
        setDetail(detailAfterStop(s));
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive, acceptState]);

  /** „Wertung beginnen“ — Probephase beenden, Scheibe leeren, gewertete Serie starten. */
  const finishProbe = useCallback(async () => {
    await runExclusive(async () => {
      try {
        const s = await api.finishProbe();
        if (!acceptState(s)) return;
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive, acceptState]);

  const resetSeries = useCallback(async () => {
    await runExclusive(async () => {
      try {
        const s = await api.resetTrainingSeries();
        if (!acceptState(s)) return;
        setDetail(detailAfterReset(s));
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive, acceptState]);

  const setEndlessMode = useCallback(
    async (endless: boolean) => {
      try {
        const s = await api.setTrainingEndless(endless);
        if (!acceptState(s)) return;
        if (endless && s.session && !s.session.endedAt) {
          setDetail("Endlosmodus — Schüsse werden nicht gespeichert");
        }
      } catch (e) {
        setDetail(String(e));
      }
    },
    [acceptState],
  );

  const setSeriesShots = useCallback(async (shots: number) => {
    try {
      const s = await api.setTrainingSeriesShots(shots);
      if (!acceptState(s)) return;
    } catch (e) {
      setDetail(String(e));
    }
  }, [acceptState]);

  const fireAt = useCallback(
    async (x: number, y: number) => {
      await runExclusive(async () => {
        try {
          const s = await api.fireAimShot(x, y);
          if (!acceptState(s)) return;
          if (s.seriesComplete) {
            const saveMsg = s.trainingSave ? trainingSaveUiMessage(s.trainingSave) : null;
            setDetail(
              saveMsg ??
                `Serie beendet — ${s.shotCount}/${s.maxShots ?? s.shotCount} Schüsse`,
            );
          }
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive, acceptState],
  );

  const fireOnce = useCallback(async () => {
    const angle = ((state?.shotCount ?? 0) * 0.9) % (Math.PI * 2);
    const r = 40 + ((state?.shotCount ?? 0) % 5) * 25;
    await fireAt(Math.cos(angle) * r, Math.sin(angle) * r);
  }, [fireAt, state?.shotCount]);

  const toggleAuto = useCallback(async () => {
    await runExclusive(async () => {
      const next = !state?.autoFire;
      try {
        await api.setAutoFire(next);
        await refresh();
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive, state?.autoFire, refresh]);

  const notify = useCallback((message: string | null) => {
    setDetail(message);
  }, []);

  return {
    state,
    detail,
    notify,
    busy,
    running: isRunning(state?.status ?? "disconnected"),
    refresh,
    startTraining,
    startEntry,
    startEntryPrepared,
    stopThenStartEntry,
    stop,
    finishProbe,
    resetSeries,
    setEndlessMode,
    setSeriesShots,
    fireOnce,
    fireAt,
    toggleAuto,
  };
}
