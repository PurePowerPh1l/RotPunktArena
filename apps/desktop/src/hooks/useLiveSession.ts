import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ConnectionUpdate, LiveState, SeriesCompletePayload, UiShot } from "@reddot/domain";
import { trainingSaveUiMessage } from "@reddot/domain";
import * as api from "../api/commands";
import { useAsyncAction } from "./useAsyncAction";

function isRunning(status: LiveState["status"]): boolean {
  return status === "searching" || status === "connected";
}

function detailAfterStop(s: LiveState): string | null {
  const saveMsg = s.trainingSave ? trainingSaveUiMessage(s.trainingSave) : null;
  if (saveMsg) return saveMsg;
  if (s.shots.length > 0 && (s.session?.competitionId || s.session?.entryId)) {
    return `Session beendet (${s.shots.length} Schüsse) — unter Verwaltung → Ergebnisse`;
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

export function useLiveSession() {
  const [state, setState] = useState<LiveState | null>(null);
  const [detail, setDetail] = useState<string | null>(null);
  const { busy, run: runExclusive } = useAsyncAction();

  const refresh = useCallback(async () => {
    const s = await api.getLiveState();
    setState(s);
  }, []);

  useEffect(() => {
    void refresh();
    let unsubs: Array<() => void> = [];
    (async () => {
      unsubs.push(
        await listen<ConnectionUpdate>("connection", (e) => {
          setDetail(e.payload.detail ?? null);
          setState((prev) =>
            prev
              ? {
                  ...prev,
                  status: e.payload.status,
                  transport: e.payload.transport,
                  port: e.payload.port,
                }
              : prev,
          );
        }),
      );
      unsubs.push(
        await listen<UiShot>("shot", (e) => {
          setState((prev) => {
            if (!prev) return prev;
            if (prev.shots.some((s) => s.shotIndex === e.payload.shotIndex)) {
              return prev;
            }
            return {
              ...prev,
              shots: [...prev.shots, e.payload],
              seriesTotal: e.payload.seriesTotal,
              seriesTeilerTotal: e.payload.seriesTeilerTotal,
              lastShot: e.payload,
              status: "connected",
            };
          });
        }),
      );
      unsubs.push(
        await listen<SeriesCompletePayload>("series_complete", async (e) => {
          const s = await api.getLiveState();
          setState(s);
          const saveMsg = s.trainingSave ? trainingSaveUiMessage(s.trainingSave) : null;
          setDetail(
            saveMsg ??
              `Serie beendet — ${e.payload.shotCount}/${e.payload.maxShots} Schüsse`,
          );
        }),
      );
    })();
    return () => {
      unsubs.forEach((u) => u());
    };
  }, [refresh]);

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
          setState(s);
          setDetail(endless ? "Endlosmodus — Schüsse werden nicht gespeichert" : null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive],
  );

  const startEntry = useCallback(
    async (entryId: string, useSimulator: boolean) => {
      await runExclusive(async () => {
        try {
          const s = await api.startEntrySession(entryId, useSimulator);
          setState(s);
          setDetail(null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive],
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
          setState(s);
          setDetail(null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive],
  );

  /** Stop (falls laufend) + nächsten Entry starten — atomar gegen Doppelklick. */
  const stopThenStartEntry = useCallback(
    async (nextEntryId: string, useSimulator: boolean) => {
      await runExclusive(async () => {
        try {
          const current = await api.getLiveState();
          if (isRunning(current.status)) {
            const ended = await api.endTraining();
            setState(ended);
            setDetail(detailAfterStop(ended));
          }
          const s = await api.startEntrySession(nextEntryId, useSimulator);
          setState(s);
          setDetail(null);
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive],
  );

  const stop = useCallback(async () => {
    await runExclusive(async () => {
      try {
        const s = await api.endTraining();
        setState(s);
        setDetail(detailAfterStop(s));
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive]);

  const resetSeries = useCallback(async () => {
    await runExclusive(async () => {
      try {
        const s = await api.resetTrainingSeries();
        setState(s);
        setDetail(detailAfterReset(s));
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive]);

  const setEndlessMode = useCallback(
    async (endless: boolean) => {
      try {
        const s = await api.setTrainingEndless(endless);
        setState(s);
        if (endless && s.session && !s.session.endedAt) {
          setDetail("Endlosmodus — Schüsse werden nicht gespeichert");
        }
      } catch (e) {
        setDetail(String(e));
      }
    },
    [],
  );

  const fireAt = useCallback(
    async (x: number, y: number) => {
      await runExclusive(async () => {
        try {
          const s = await api.fireAimShot(x, y);
          setState(s);
          if (s.seriesComplete) {
            const saveMsg = s.trainingSave ? trainingSaveUiMessage(s.trainingSave) : null;
            setDetail(
              saveMsg ??
                `Serie beendet — ${s.shots.length}/${s.maxShots ?? s.shots.length} Schüsse`,
            );
          }
        } catch (e) {
          setDetail(String(e));
        }
      });
    },
    [runExclusive],
  );

  const fireOnce = useCallback(async () => {
    const angle = ((state?.shots.length ?? 0) * 0.9) % (Math.PI * 2);
    const r = 40 + ((state?.shots.length ?? 0) % 5) * 25;
    await fireAt(Math.cos(angle) * r, Math.sin(angle) * r);
  }, [fireAt, state?.shots.length]);

  const toggleAuto = useCallback(async () => {
    await runExclusive(async () => {
      const next = !state?.autoFire;
      try {
        await api.setAutoFire(next);
        setState((prev) => (prev ? { ...prev, autoFire: next } : prev));
      } catch (e) {
        setDetail(String(e));
      }
    });
  }, [runExclusive, state?.autoFire]);

  return {
    state,
    detail,
    busy,
    running: isRunning(state?.status ?? "disconnected"),
    refresh,
    startTraining,
    startEntry,
    startEntryPrepared,
    stopThenStartEntry,
    stop,
    resetSeries,
    setEndlessMode,
    fireOnce,
    fireAt,
    toggleAuto,
  };
}
