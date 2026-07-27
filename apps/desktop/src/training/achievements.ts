import type { TrainingSessionSummary } from "@rotpunktarena/domain";
import { levelFromXp, xpFromSessions } from "./stats";

/**
 * UI-only achievement badges from saved training series.
 * Display gamification only (§10 exception) — never recomputes shot scores.
 * See docs/code-guidelines.md §10.
 */

export type AchievementId =
  | "first_series"
  | "series_5"
  | "series_25"
  | "series_50"
  | "shots_100"
  | "shots_500"
  | "shots_1000"
  | "serie_100"
  | "serie_200"
  | "serie_300"
  | "teiler_100"
  | "teiler_50"
  | "teiler_20"
  | "level_5"
  | "level_10"
  | "streak_3"
  | "hot_day";

export type AchievementDef = {
  id: AchievementId;
  title: string;
  description: string;
};

export type AchievementStatus = AchievementDef & {
  unlocked: boolean;
  /** 0–1 progress toward unlock (1 if unlocked). */
  progress: number;
};

export const ACHIEVEMENT_DEFS: readonly AchievementDef[] = [
  {
    id: "first_series",
    title: "Erster Schuss",
    description: "Erste Trainingsserie speichern",
  },
  {
    id: "series_5",
    title: "Warmgeschossen",
    description: "5 Serien absolviert",
  },
  {
    id: "series_25",
    title: "Stammgast",
    description: "25 Serien absolviert",
  },
  {
    id: "series_50",
    title: "Eisen im Blut",
    description: "50 Serien absolviert",
  },
  {
    id: "shots_100",
    title: "Centurion",
    description: "100 Schüsse insgesamt",
  },
  {
    id: "shots_500",
    title: "Dauerbrenner",
    description: "500 Schüsse insgesamt",
  },
  {
    id: "shots_1000",
    title: "Tausendsassa",
    description: "1.000 Schüsse insgesamt",
  },
  {
    id: "serie_100",
    title: "Hunderter",
    description: "Eine Serie mit ≥ 100 Punkten",
  },
  {
    id: "serie_200",
    title: "Doppelpack",
    description: "Eine Serie mit ≥ 200 Punkten",
  },
  {
    id: "serie_300",
    title: "Hochform",
    description: "Eine Serie mit ≥ 300 Punkten",
  },
  {
    id: "teiler_100",
    title: "Nah dran",
    description: "Ø Teiler unter 100 in einer Serie",
  },
  {
    id: "teiler_50",
    title: "Präzision",
    description: "Ø Teiler unter 50 in einer Serie",
  },
  {
    id: "teiler_20",
    title: "Ins Schwarze",
    description: "Ø Teiler unter 20 in einer Serie",
  },
  {
    id: "level_5",
    title: "Aufsteiger",
    description: "Level 5 erreichen",
  },
  {
    id: "level_10",
    title: "Legendenanwärter",
    description: "Level 10 erreichen",
  },
  {
    id: "streak_3",
    title: "Aufwärtstrend",
    description: "3 Serien in Folge mit steigenden Punkten",
  },
  {
    id: "hot_day",
    title: "Trainingstag",
    description: "5 Serien an einem Kalendertag",
  },
] as const;

function ratio(current: number, target: number): number {
  if (target <= 0) return 1;
  return Math.min(1, Math.max(0, current / target));
}

function hasImprovingStreak(sessions: TrainingSessionSummary[], len: number): boolean {
  if (sessions.length < len) return false;
  for (let i = len - 1; i < sessions.length; i++) {
    let ok = true;
    for (let j = i - len + 2; j <= i; j++) {
      if (sessions[j]!.punkteTotal <= sessions[j - 1]!.punkteTotal) {
        ok = false;
        break;
      }
    }
    if (ok) return true;
  }
  return false;
}

function maxSameDayCount(sessions: TrainingSessionSummary[]): number {
  const byDay = new Map<string, number>();
  let max = 0;
  for (const s of sessions) {
    const d = new Date(s.endedAt);
    if (!Number.isFinite(d.getTime())) continue;
    const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
    const next = (byDay.get(key) ?? 0) + 1;
    byDay.set(key, next);
    if (next > max) max = next;
  }
  return max;
}

/** Evaluate achievements from chronological sessions (oldest → newest). */
export function evaluateAchievements(
  sessions: TrainingSessionSummary[],
): AchievementStatus[] {
  const sessionCount = sessions.length;
  const shotCount = sessions.reduce((a, s) => a + Math.max(0, s.shotCount), 0);
  const bestSerie = sessions.length
    ? Math.max(...sessions.map((s) => s.punkteTotal))
    : 0;
  const bestTeiler = sessions.length
    ? Math.min(...sessions.map((s) => s.teilerAvg).filter((t) => Number.isFinite(t)))
    : Number.POSITIVE_INFINITY;
  const level = levelFromXp(xpFromSessions(sessions)).level;
  const streakOk = hasImprovingStreak(sessions, 3);
  const hotDay = maxSameDayCount(sessions);

  const progressOf = (id: AchievementId): { unlocked: boolean; progress: number } => {
    switch (id) {
      case "first_series":
        return { unlocked: sessionCount >= 1, progress: ratio(sessionCount, 1) };
      case "series_5":
        return { unlocked: sessionCount >= 5, progress: ratio(sessionCount, 5) };
      case "series_25":
        return { unlocked: sessionCount >= 25, progress: ratio(sessionCount, 25) };
      case "series_50":
        return { unlocked: sessionCount >= 50, progress: ratio(sessionCount, 50) };
      case "shots_100":
        return { unlocked: shotCount >= 100, progress: ratio(shotCount, 100) };
      case "shots_500":
        return { unlocked: shotCount >= 500, progress: ratio(shotCount, 500) };
      case "shots_1000":
        return { unlocked: shotCount >= 1000, progress: ratio(shotCount, 1000) };
      case "serie_100":
        return { unlocked: bestSerie >= 100, progress: ratio(bestSerie, 100) };
      case "serie_200":
        return { unlocked: bestSerie >= 200, progress: ratio(bestSerie, 200) };
      case "serie_300":
        return { unlocked: bestSerie >= 300, progress: ratio(bestSerie, 300) };
      case "teiler_100":
        return {
          unlocked: bestTeiler < 100,
          progress: bestTeiler === Number.POSITIVE_INFINITY ? 0 : ratio(100 / Math.max(bestTeiler, 1), 1),
        };
      case "teiler_50":
        return {
          unlocked: bestTeiler < 50,
          progress: bestTeiler === Number.POSITIVE_INFINITY ? 0 : ratio(50 / Math.max(bestTeiler, 1), 1),
        };
      case "teiler_20":
        return {
          unlocked: bestTeiler < 20,
          progress: bestTeiler === Number.POSITIVE_INFINITY ? 0 : ratio(20 / Math.max(bestTeiler, 1), 1),
        };
      case "level_5":
        return { unlocked: level >= 5, progress: ratio(level, 5) };
      case "level_10":
        return { unlocked: level >= 10, progress: ratio(level, 10) };
      case "streak_3":
        return { unlocked: streakOk, progress: streakOk ? 1 : ratio(sessionCount, 3) };
      case "hot_day":
        return { unlocked: hotDay >= 5, progress: ratio(hotDay, 5) };
      default:
        return { unlocked: false, progress: 0 };
    }
  };

  return ACHIEVEMENT_DEFS.map((def) => {
    const { unlocked, progress } = progressOf(def.id);
    return { ...def, unlocked, progress: unlocked ? 1 : progress };
  });
}
