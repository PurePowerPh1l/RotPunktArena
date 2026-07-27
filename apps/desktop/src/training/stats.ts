import type { TrainingSessionSummary } from "@rotpunktarena/domain";

/**
 * Display aggregates from saved series — not shot scoring logic.
 * Soft XP/level titles are UI gamification (§10 exception); series totals
 * come from Rust-persisted TrainingSessionSummary. See docs/code-guidelines.md §10.
 */
export type TrainingStats = {
  sessionCount: number;
  shotCount: number;
  avgSeriePunkte: number;
  avgPunkteProSchuss: number;
  avgTeiler: number;
  bestSerie: number;
  bestTeiler: number;
  lastSerie: number | null;
  /** Recent window avg minus earlier window (punkte). Positive = better. */
  trendPunkte: number | null;
  /** Recent teiler avg minus earlier (negative = better for teiler). */
  trendTeiler: number | null;
  level: number;
  levelTitle: string;
  levelProgress: number;
  xp: number;
  xpIntoLevel: number;
  xpForLevel: number;
  xpToNext: number;
};

const LEVEL_TITLES = [
  "Neuling",
  "Schütze",
  "Sicher",
  "Treffsicher",
  "Präzise",
  "Scharfschütze",
  "Elite",
  "Meister",
  "Champion",
  "Legende",
] as const;

/** Soft XP curve from existing series totals. */
export function xpFromSessions(sessions: TrainingSessionSummary[]): number {
  return sessions.reduce((sum, s) => {
    const shots = Math.max(0, s.shotCount);
    const punkte = Math.max(0, s.punkteTotal);
    return sum + punkte + shots * 1.5;
  }, 0);
}

export function levelFromXp(xp: number): {
  level: number;
  title: string;
  progress: number;
  xpIntoLevel: number;
  xpForLevel: number;
  xpToNext: number;
} {
  // Level n starts at ~ n² * 40
  let level = 1;
  let spent = 0;
  while (level < 99) {
    const need = 40 * level * level;
    if (xp < spent + need) {
      const into = xp - spent;
      return {
        level,
        title: LEVEL_TITLES[Math.min(level - 1, LEVEL_TITLES.length - 1)]!,
        progress: need <= 0 ? 1 : Math.min(1, into / need),
        xpIntoLevel: into,
        xpForLevel: need,
        xpToNext: Math.max(0, need - into),
      };
    }
    spent += need;
    level += 1;
  }
  return {
    level: 99,
    title: LEVEL_TITLES[LEVEL_TITLES.length - 1]!,
    progress: 1,
    xpIntoLevel: 0,
    xpForLevel: 1,
    xpToNext: 0,
  };
}

function avg(nums: number[]): number {
  if (nums.length === 0) return 0;
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

/** `sessions` chronological oldest → newest. */
export function computeTrainingStats(
  sessions: TrainingSessionSummary[],
): TrainingStats {
  const empty: TrainingStats = {
    sessionCount: 0,
    shotCount: 0,
    avgSeriePunkte: 0,
    avgPunkteProSchuss: 0,
    avgTeiler: 0,
    bestSerie: 0,
    bestTeiler: 0,
    lastSerie: null,
    trendPunkte: null,
    trendTeiler: null,
    level: 1,
    levelTitle: LEVEL_TITLES[0],
    levelProgress: 0,
    xp: 0,
    xpIntoLevel: 0,
    xpForLevel: 40,
    xpToNext: 40,
  };
  if (sessions.length === 0) return empty;

  const punkte = sessions.map((s) => s.punkteTotal);
  const teiler = sessions.map((s) => s.teilerAvg);
  const shotCount = sessions.reduce((a, s) => a + s.shotCount, 0);
  const punkteSum = sessions.reduce((a, s) => a + s.punkteTotal, 0);

  const window = Math.min(5, Math.max(1, Math.floor(sessions.length / 2)));
  let trendPunkte: number | null = null;
  let trendTeiler: number | null = null;
  if (sessions.length >= 4) {
    const recent = sessions.slice(-window);
    const earlier = sessions.slice(0, Math.min(window, sessions.length - window));
    if (earlier.length > 0) {
      trendPunkte = avg(recent.map((s) => s.punkteTotal)) - avg(earlier.map((s) => s.punkteTotal));
      trendTeiler = avg(recent.map((s) => s.teilerAvg)) - avg(earlier.map((s) => s.teilerAvg));
    }
  }

  const xp = xpFromSessions(sessions);
  const lvl = levelFromXp(xp);

  return {
    sessionCount: sessions.length,
    shotCount,
    avgSeriePunkte: avg(punkte),
    avgPunkteProSchuss: shotCount > 0 ? punkteSum / shotCount : 0,
    avgTeiler: avg(teiler),
    bestSerie: Math.max(...punkte),
    bestTeiler: Math.min(...teiler),
    lastSerie: punkte[punkte.length - 1] ?? null,
    trendPunkte,
    trendTeiler,
    level: lvl.level,
    levelTitle: lvl.title,
    levelProgress: lvl.progress,
    xp,
    xpIntoLevel: lvl.xpIntoLevel,
    xpForLevel: lvl.xpForLevel,
    xpToNext: lvl.xpToNext,
  };
}

export function fmtStat(v: number, digits = 1): string {
  if (!Number.isFinite(v)) return "—";
  if (Number.isInteger(v) && digits === 0) return String(v);
  return v.toLocaleString("de-DE", {
    maximumFractionDigits: digits,
    minimumFractionDigits: Number.isInteger(v) ? 0 : Math.min(digits, 1),
  });
}

export function fmtDelta(v: number | null, invert = false): {
  text: string;
  kind: "up" | "down" | "flat" | "none";
} {
  if (v == null || !Number.isFinite(v)) return { text: "—", kind: "none" };
  if (Math.abs(v) < 0.05) return { text: "±0", kind: "flat" };
  const better = invert ? v < 0 : v > 0;
  const sign = v > 0 ? "+" : "";
  return {
    text: `${sign}${fmtStat(v)}`,
    kind: better ? "up" : "down",
  };
}
