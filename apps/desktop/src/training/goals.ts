import { tenShotPoints } from "./comparison";
import type { TrainingSessionSummary } from "@rotpunktarena/domain";
import { filterSessionsByWindow, type HistoryWindowDays } from "./insights";
import { fmtStat } from "./stats";

export type GoalKind = "avgSerie" | "avgTeiler" | "seriesCount" | "bestSerie";

export type TrainingGoal = {
  id: string;
  legacySeriesTarget?: boolean;
  kind: GoalKind;
  target: number;
  /** Evaluate against this window; null = all loaded sessions. */
  windowDays: HistoryWindowDays;
};

export type GoalProgress = {
  goal: TrainingGoal;
  current: number;
  ratio: number;
  label: string;
  unit: string;
  done: boolean;
};

const STORAGE_PREFIX = "reddot.trainingGoals.v2:";

export const GOAL_KIND_OPTIONS: {
  value: GoalKind;
  label: string;
  unit: string;
}[] = [
  { value: "avgSerie", label: "Ø Leistung (10 Schuss)", unit: "Punkte" },
  { value: "bestSerie", label: "Beste Leistung (10 Schuss)", unit: "Punkte" },
  { value: "avgTeiler", label: "Ø Teiler", unit: "Teiler" },
  { value: "seriesCount", label: "Anzahl Serien", unit: "Serien" },
];

function storageKey(filterKey: string): string {
  return `${STORAGE_PREFIX}${filterKey || "all"}`;
}

function isGoalKind(v: unknown): v is GoalKind {
  return GOAL_KIND_OPTIONS.some((o) => o.value === v);
}

export function loadGoals(filterKey: string): TrainingGoal[] {
  try {
    const current = localStorage.getItem(storageKey(filterKey));
    const raw = current ?? localStorage.getItem(`reddot.trainingGoals.v1:${filterKey || "all"}`);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as TrainingGoal[];
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (g) =>
        g &&
        typeof g.id === "string" &&
        typeof g.target === "number" && Number.isFinite(g.target) && g.target > 0 &&
        (g.windowDays == null || [7,30,90].includes(g.windowDays)) &&
        isGoalKind(g.kind),
    ).slice(0,4).map((g) => !current && (g.kind === "avgSerie" || g.kind === "bestSerie") ? {...g, legacySeriesTarget:true} : g);
  } catch {
    return [];
  }
}

export function saveGoals(filterKey: string, goals: TrainingGoal[]): void {
  localStorage.setItem(storageKey(filterKey), JSON.stringify(goals.slice(0, 4)));
}

export function createGoal(
  kind: GoalKind,
  target: number,
  windowDays: HistoryWindowDays = 30,
): TrainingGoal {
  return {
    id: `${kind}-${Date.now().toString(36)}`,
    kind,
    target,
    windowDays,
  };
}

function currentForGoal(
  sessions: TrainingSessionSummary[],
  goal: TrainingGoal,
): number {
  const scoped = filterSessionsByWindow(sessions, goal.windowDays);
  if (scoped.length === 0) return 0;
  if (goal.kind === "seriesCount") return scoped.length;
  if (goal.kind === "bestSerie") {
    return Math.max(...scoped.map(tenShotPoints));
  }
  if (goal.kind === "avgTeiler") {
    return scoped.reduce((a, s) => a + s.teilerAvg, 0) / scoped.length;
  }
  return scoped.reduce((a, s) => a + tenShotPoints(s), 0) / scoped.length;
}

export function evaluateGoals(
  sessions: TrainingSessionSummary[],
  goals: TrainingGoal[],
): GoalProgress[] {
  return goals.map((goal) => {
    const meta = GOAL_KIND_OPTIONS.find((o) => o.value === goal.kind)!;
    const current = currentForGoal(sessions, goal);
    const invert = goal.kind === "avgTeiler";
    const ratio = invert
      ? goal.target > 0
        ? Math.min(1, goal.target / Math.max(current, 0.01))
        : 0
      : goal.target > 0
        ? Math.min(1, current / goal.target)
        : 0;
    const done = invert
      ? current > 0 && current <= goal.target
      : current >= goal.target;
    const windowLabel =
      goal.windowDays == null ? "alle" : `${goal.windowDays}d`;
    return {
      goal,
      current,
      ratio,
      label: `${meta.label} · ${windowLabel}`,
      unit: meta.unit,
      done,
    };
  });
}

export function formatGoalValue(kind: GoalKind, v: number): string {
  if (kind === "seriesCount") return fmtStat(v, 0);
  return fmtStat(v);
}

/** Contains only this app's versioned training goals, never arbitrary browser storage. */
export function exportTrainingGoals(): string {
  const entries: Record<string, unknown> = {};
  for (let index = 0; index < localStorage.length; index++) {
    const key = localStorage.key(index);
    if (key?.startsWith("reddot.trainingGoals.v")) {
      const raw = localStorage.getItem(key);
      if (raw) { try { entries[key] = JSON.parse(raw); } catch { /* retain other valid goals */ } }
    }
  }
  return JSON.stringify({ formatVersion: 2, comparison: "tenShotPoints", entries });
}
