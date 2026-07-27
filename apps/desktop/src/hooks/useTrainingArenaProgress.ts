import { useCallback, useEffect, useRef, useState } from "react";
import type { TrainingSaveInfo, TrainingSessionSummary } from "@rotpunktarena/domain";
import type { ShooterValue } from "../components/ShooterAutocomplete";
import * as api from "../api/commands";
import { createRequestSeq } from "../lib/requestSeq";
import {
  computeSeriesPulse,
  pickEigenRival,
  progressFromSessions,
  type SeriesPulse,
  type TrainingProgressSnapshot,
} from "../training/seriesPulse";

type Args = {
  enabled: boolean;
  shooter: ShooterValue;
  seriesComplete: boolean;
  trainingSave: TrainingSaveInfo | null | undefined;
  /** When true, compare pulse/strip target against eigene letzte Serie. */
  rivalEnabled: boolean;
};

export type TrainingArenaProgress = {
  progress: TrainingProgressSnapshot | null;
  pulse: SeriesPulse | null;
  rivalTarget: { label: string; punkte: number } | null;
  refresh: () => void;
  clearPulse: () => void;
};

function historyFilter(shooter: ShooterValue) {
  const name = shooter.name.trim();
  if (!name && !shooter.personId) return null;
  return {
    personId: shooter.personId ?? null,
    shooterName: shooter.personId ? null : name || null,
  };
}

/**
 * Training-only: load soft XP/Liga for the strip and build a post-series pulse
 * when a full series was saved.
 */
export function useTrainingArenaProgress({
  enabled,
  shooter,
  seriesComplete,
  trainingSave,
  rivalEnabled,
}: Args): TrainingArenaProgress {
  const [sessions, setSessions] = useState<TrainingSessionSummary[]>([]);
  const [pulse, setPulse] = useState<SeriesPulse | null>(null);
  const seq = useRef(createRequestSeq()).current;
  const pulseKey = useRef<string | null>(null);

  const load = useCallback(async () => {
    if (!enabled) {
      setSessions([]);
      return;
    }
    const filter = historyFilter(shooter);
    if (!filter) {
      setSessions([]);
      return;
    }
    const token = seq.begin();
    try {
      const list = await api.listTrainingHistory(80, filter);
      if (!seq.isCurrent(token)) return;
      setSessions(list);
    } catch {
      if (seq.isCurrent(token)) setSessions([]);
    }
  }, [enabled, shooter, seq]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!enabled) {
      setPulse(null);
      pulseKey.current = null;
      return;
    }
    if (!seriesComplete) {
      setPulse(null);
      pulseKey.current = null;
      return;
    }
    if (!trainingSave?.saved) return;

    const key = `${trainingSave.shotCount}:${trainingSave.reason}:${shooter.personId ?? shooter.name}`;
    if (pulseKey.current === key) return;
    pulseKey.current = key;

    let cancelled = false;
    (async () => {
      const filter = historyFilter(shooter);
      if (!filter) return;
      try {
        const list = await api.listTrainingHistory(80, filter);
        if (cancelled) return;
        setSessions(list);
        const rival = rivalEnabled ? pickEigenRival(list.slice(0, -1)) : null;
        setPulse(computeSeriesPulse(list, rival));
      } catch {
        if (!cancelled) setPulse(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [
    enabled,
    seriesComplete,
    trainingSave?.saved,
    trainingSave?.shotCount,
    trainingSave?.reason,
    shooter,
    rivalEnabled,
  ]);

  const progress =
    enabled && sessions.length >= 0 && historyFilter(shooter)
      ? progressFromSessions(sessions)
      : null;

  const rivalTarget =
    rivalEnabled && sessions.length > 0
      ? pickEigenRival(
          seriesComplete && trainingSave?.saved
            ? sessions.slice(0, -1)
            : sessions,
        )
      : null;

  const refresh = useCallback(() => {
    void load();
  }, [load]);

  const clearPulse = useCallback(() => {
    setPulse(null);
  }, []);

  return {
    progress: historyFilter(shooter) ? progress : null,
    pulse,
    rivalTarget,
    refresh,
    clearPulse,
  };
}
