import { rankFromSr } from "../training/league";
import { levelFromXp } from "../training/stats";
import { TRAINING_HISTORY_WINDOW } from "../training/comparison";
import { useCallback, useEffect, useRef, useState } from "react";
import type { TrainingSaveInfo, TrainingSessionSummary } from "@rotpunktarena/domain";
import type { ShooterValue } from "../components/ShooterAutocomplete";
import * as api from "../api/commands";
import { createRequestSeq } from "../lib/requestSeq";
import {
  computeSeriesPulse,
  applyLifetimeToPulse,
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
  const [lifetimes, setLifetimes] = useState<api.TrainingLifetime[]>([]);
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
      const [list, totals] = await Promise.all([api.listTrainingHistory(TRAINING_HISTORY_WINDOW, filter), api.getTrainingLifetime()]);
      if (!seq.isCurrent(token)) return;
      setSessions(list); setLifetimes(totals);
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
        const [list, totals] = await Promise.all([api.listTrainingHistory(TRAINING_HISTORY_WINDOW, filter), api.getTrainingLifetime()]);
        if (cancelled) return;
        setSessions(list); setLifetimes(totals);
        const rival = rivalEnabled ? pickEigenRival(list.slice(0, -1)) : null;
        const pulse = computeSeriesPulse(list, rival);
        const key = shooter.personId ? `id:${shooter.personId}` : `name:${shooter.name.trim().toLowerCase()}`;
        const total = totals.find((t) => t.key === key);
        setPulse(applyLifetimeToPulse(pulse, total));
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

  const key = shooter.personId ? `id:${shooter.personId}` : `name:${shooter.name.trim().toLowerCase()}`;
  const total = lifetimes.find((s) => s.key === key);
  if (progress && total) {
    const xp = total.pointsTotal + total.shotCount*1.5;
    const level = levelFromXp(xp);
    progress.league = rankFromSr(total.sr,total.sessionCount);
    progress.sessionCount = total.sessionCount;
    Object.assign(progress.stats,{xp,shotCount:total.shotCount,sessionCount:total.sessionCount,level:level.level,levelTitle:level.title,levelProgress:level.progress,xpIntoLevel:level.xpIntoLevel,xpForLevel:level.xpForLevel,xpToNext:level.xpToNext});
  }

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
