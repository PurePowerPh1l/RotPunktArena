import type { TrainingSessionSummary, UiShot } from "@rotpunktarena/domain";
import { sessionsChronological } from "./seriesPulse";
import { fmtStat } from "./stats";

export type HistoryWindowDays = 7 | 30 | 90 | null;

export const HISTORY_WINDOW_OPTIONS: {
  value: HistoryWindowDays;
  label: string;
}[] = [
  { value: 7, label: "7 Tage" },
  { value: 30, label: "30 Tage" },
  { value: 90, label: "90 Tage" },
  { value: null, label: "Alle" },
];

export function filterSessionsByWindow(
  sessions: TrainingSessionSummary[],
  windowDays: HistoryWindowDays,
  now = Date.now(),
): TrainingSessionSummary[] {
  if (windowDays == null) return sessions;
  const cutoff = now - windowDays * 24 * 60 * 60 * 1000;
  return sessions.filter((s) => {
    const t = Date.parse(s.endedAt);
    return Number.isFinite(t) && t >= cutoff;
  });
}

export type FormInsight = {
  id: string;
  label: string;
  value: string;
  kind: "up" | "down" | "flat" | "info";
};

export type CompareBanner = {
  lastPunkte: number;
  avg5: number | null;
  avg10: number | null;
  bestWeek: number | null;
  deltaVs5: number | null;
  hint: string;
  kind: "up" | "down" | "flat";
};

function avg(nums: number[]): number {
  if (nums.length === 0) return 0;
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

/** Last series vs recent averages + best of calendar week. */
export function computeCompareBanner(
  sessions: TrainingSessionSummary[],
  now = new Date(),
): CompareBanner | null {
  const chrono = sessionsChronological(sessions);
  if (chrono.length === 0) return null;
  const last = chrono[chrono.length - 1]!;
  const prior = chrono.slice(0, -1);
  const avg5 =
    prior.length > 0
      ? avg(prior.slice(-5).map((s) => s.punkteTotal))
      : null;
  const avg10 =
    prior.length > 0
      ? avg(prior.slice(-10).map((s) => s.punkteTotal))
      : null;

  const weekStart = new Date(now);
  weekStart.setHours(0, 0, 0, 0);
  const day = (weekStart.getDay() + 6) % 7; // Monday=0
  weekStart.setDate(weekStart.getDate() - day);
  const weekMs = weekStart.getTime();
  const weekSeries = chrono.filter((s) => Date.parse(s.endedAt) >= weekMs);
  const bestWeek =
    weekSeries.length > 0
      ? Math.max(...weekSeries.map((s) => s.punkteTotal))
      : null;

  const deltaVs5 =
    avg5 != null ? last.punkteTotal - avg5 : null;
  let kind: CompareBanner["kind"] = "flat";
  let hint = "erste Serie im Fenster";
  if (deltaVs5 != null) {
    if (Math.abs(deltaVs5) < 0.5) {
      kind = "flat";
      hint = "im Rahmen deines 5er-Schnitts";
    } else if (deltaVs5 > 0) {
      kind = "up";
      hint = "über dem 5er-Schnitt";
    } else {
      kind = "down";
      hint = "unter dem 5er-Schnitt";
    }
  }

  return {
    lastPunkte: last.punkteTotal,
    avg5,
    avg10,
    bestWeek,
    deltaVs5,
    hint,
    kind,
  };
}

/** Soft form chips from series aggregates (+ optional last-series shots). */
export function computeFormInsights(
  sessions: TrainingSessionSummary[],
  lastShots?: UiShot[] | null,
): FormInsight[] {
  const chrono = sessionsChronological(sessions);
  const out: FormInsight[] = [];
  if (chrono.length === 0) return out;

  const punkte = chrono.map((s) => s.punkteTotal);
  const mean = avg(punkte);
  let above = 0;
  for (let i = chrono.length - 1; i >= 0; i--) {
    if ((chrono[i]?.punkteTotal ?? 0) >= mean) above += 1;
    else break;
  }
  if (above >= 2) {
    out.push({
      id: "above-avg",
      label: "Form",
      value: `${above} Serien über Schnitt`,
      kind: "up",
    });
  }

  let below = 0;
  for (let i = chrono.length - 1; i >= 0; i--) {
    if ((chrono[i]?.punkteTotal ?? 0) < mean) below += 1;
    else break;
  }
  if (below >= 2) {
    out.push({
      id: "below-avg",
      label: "Form",
      value: `${below} Serien unter Schnitt`,
      kind: "down",
    });
  }

  if (chrono.length >= 3) {
    const last3 = avg(chrono.slice(-3).map((s) => s.punkteTotal));
    const prior = chrono.slice(0, -3);
    if (prior.length > 0) {
      const base = avg(prior.slice(-5).map((s) => s.punkteTotal));
      const d = last3 - base;
      if (Math.abs(d) >= 0.5) {
        out.push({
          id: "momentum",
          label: "Momentum",
          value:
            d > 0
              ? `+${fmtStat(d)} vs. davor`
              : `${fmtStat(d)} vs. davor`,
          kind: d > 0 ? "up" : "down",
        });
      }
    }
  }

  if (lastShots && lastShots.length > 0) {
    let missStreak = 0;
    for (let i = lastShots.length - 1; i >= 0; i--) {
      if ((lastShots[i]?.valueDisplay ?? 0) <= 0) break;
      missStreak += 1;
    }
    if (missStreak >= 3 && missStreak === lastShots.length) {
      out.push({
        id: "clean",
        label: "Sauber",
        value: `kein Miss · ${missStreak} Schüsse`,
        kind: "up",
      });
    } else if (missStreak >= 3) {
      out.push({
        id: "no-miss",
        label: "Serie",
        value: `kein Miss seit ${missStreak}`,
        kind: "up",
      });
    }

    let tenStreak = 0;
    for (let i = lastShots.length - 1; i >= 0; i--) {
      if ((lastShots[i]?.valueDisplay ?? 0) < 10) break;
      tenStreak += 1;
    }
    if (tenStreak >= 2) {
      out.push({
        id: "tens",
        label: "Zehner",
        value: `${tenStreak} in Folge (letzte Serie)`,
        kind: "up",
      });
    }

    let bestTens = 0;
    let run = 0;
    for (const s of lastShots) {
      if (s.valueDisplay >= 10) {
        run += 1;
        bestTens = Math.max(bestTens, run);
      } else {
        run = 0;
      }
    }
    if (bestTens >= 3) {
      out.push({
        id: "best-tens",
        label: "Beste Zehner",
        value: `${bestTens} am Stück`,
        kind: "info",
      });
    }
  }

  return out.slice(0, 5);
}

export function shareSummaryText(input: {
  shooterName: string;
  punkteTotal: number;
  shotCount: number;
  teilerAvg: number;
  xpGained?: number | null;
  placeHint?: string | null;
  endedAt?: string | null;
}): string {
  const lines = [
    `RedDot — ${input.shooterName}`,
    `Serie ${fmtStat(input.punkteTotal)} Punkte · ${input.shotCount} Schüsse`,
    `Ø Teiler ${fmtStat(input.teilerAvg)}`,
  ];
  if (input.xpGained != null && input.xpGained > 0) {
    lines.push(`+${Math.round(input.xpGained)} XP`);
  }
  if (input.placeHint) lines.push(input.placeHint);
  if (input.endedAt) {
    lines.push(new Date(input.endedAt).toLocaleString("de-DE"));
  }
  return lines.join("\n");
}
