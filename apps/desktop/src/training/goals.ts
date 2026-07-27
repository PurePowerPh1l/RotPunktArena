import type { TrainingSessionSummary } from "@reddot/domain";
import { filterSessionsByWindow, type HistoryWindowDays } from "./insights";
import { fmtStat } from "./stats";

export type GoalKind = "avgSerie" | "avgTeiler" | "seriesCount" | "bestSerie";

export type TrainingGoal = {
  id: string;
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

const STORAGE_PREFIX = "reddot.trainingGoals.v1:";

export const GOAL_KIND_OPTIONS: {
  value: GoalKind;
  label: string;
  unit: string;
}[] = [
  { value: "avgSerie", label: "Ø Serie (Punkte)", unit: "Punkte" },
  { value: "bestSerie", label: "Beste Serie", unit: "Punkte" },
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
    const raw = localStorage.getItem(storageKey(filterKey));
    if (!raw) return [];
    const parsed = JSON.parse(raw) as TrainingGoal[];
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (g) =>
        g &&
        typeof g.id === "string" &&
        typeof g.target === "number" &&
        isGoalKind(g.kind),
    );
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
    return Math.max(...scoped.map((s) => s.punkteTotal));
  }
  if (goal.kind === "avgTeiler") {
    return scoped.reduce((a, s) => a + s.teilerAvg, 0) / scoped.length;
  }
  return scoped.reduce((a, s) => a + s.punkteTotal, 0) / scoped.length;
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
