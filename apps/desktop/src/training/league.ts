import type { TrainingSessionSummary } from "@rotpunktarena/domain";

/**
 * UI-only competitive league (parallel to soft XP levels).
 *
 * Intentional exception to code-guidelines §10: this is display gamification
 * derived from already-scored TrainingSessionSummary rows. It must never
 * recompute shot scores, session limits, or competition ranking — those stay
 * in Rust. See docs/code-guidelines.md §10.
 *
 * After 5 placement series, SR is seeded from that average, then each
 * further series is compared to the expected score for the current SR
 * (Elo-style). Top tiers also need a minimum series count.
 */

export type LeagueTierId =
  | "unranked"
  | "bronze"
  | "silver"
  | "gold"
  | "platinum"
  | "diamond"
  | "master"
  | "grandmaster"
  | "champion";

export type LeagueDivision = 1 | 2 | 3;

export type LeagueRank = {
  tier: LeagueTierId;
  /** Metal tiers + Meister: 3 = lowest, 1 = highest. */
  division: LeagueDivision | null;
  label: string;
  shortLabel: string;
  sr: number;
  /** Progress within current tier/division band, 0–1. */
  progress: number;
  /** Series still needed before a tier is assigned. */
  placementLeft: number;
};

const PLACEMENT_SERIES = 5;
const SR_MIN = 0;
const SR_MAX = 5000;
const K_FACTOR = 32;

/** Expected 10er-series points for SR (≈75 @0 → ≈110 @5000). */
const EXPECT_FLOOR = 75;
const EXPECT_SPAN = 35;

/** Soft volume gates — performance alone is not enough for the top. */
const TIER_MIN_SERIES: Partial<Record<LeagueTierId, number>> = {
  gold: 12,
  platinum: 22,
  diamond: 35,
  master: 50,
  grandmaster: 70,
  champion: 90,
};

const TIER_BANDS: { id: LeagueTierId; min: number; label: string }[] = [
  { id: "bronze", min: 0, label: "Bronze" },
  { id: "silver", min: 1000, label: "Silber" },
  { id: "gold", min: 1500, label: "Gold" },
  { id: "platinum", min: 2000, label: "Platin" },
  { id: "diamond", min: 2500, label: "Diamant" },
  { id: "master", min: 3000, label: "Meister" },
  { id: "grandmaster", min: 3600, label: "Großmeister" },
  { id: "champion", min: 4200, label: "Champion" },
];

export function expectedSeriePoints(sr: number): number {
  const t = Math.min(1, Math.max(0, sr / SR_MAX));
  return EXPECT_FLOOR + t * EXPECT_SPAN;
}

/** Inverse of {@link expectedSeriePoints} — used to seed placement. */
export function pointsToSr(seriePunkte: number): number {
  const t = (seriePunkte - EXPECT_FLOOR) / EXPECT_SPAN;
  return clampSr(t * SR_MAX);
}

function clampSr(sr: number): number {
  return Math.min(SR_MAX, Math.max(SR_MIN, sr));
}

export function srDelta(sr: number, seriePunkte: number): number {
  const expected = expectedSeriePoints(sr);
  const raw = K_FACTOR * Math.tanh((seriePunkte - expected) / 7);
  // Climbing gets harder near the ceiling; demotions slightly softer.
  if (raw > 0) {
    const damp = 1 - sr / (SR_MAX + 2200);
    return raw * Math.max(0.32, damp);
  }
  return raw * 0.78;
}

export function applySerieToSr(sr: number, seriePunkte: number): number {
  return clampSr(sr + srDelta(sr, Math.max(0, seriePunkte)));
}

function tierAt(sr: number): {
  id: LeagueTierId;
  label: string;
  min: number;
  nextMin: number;
} {
  let current = TIER_BANDS[0]!;
  for (const band of TIER_BANDS) {
    if (sr >= band.min) current = band;
  }
  const idx = TIER_BANDS.findIndex((b) => b.id === current.id);
  const nextMin = TIER_BANDS[idx + 1]?.min ?? SR_MAX;
  return { id: current.id, label: current.label, min: current.min, nextMin };
}

function divisionInBand(
  sr: number,
  min: number,
  nextMin: number,
): LeagueDivision | null {
  const width = nextMin - min;
  if (width <= 0) return null;
  // Großmeister / Champion: no roman divisions.
  if (min >= 3600) return null;
  const t = (sr - min) / width;
  if (t < 1 / 3) return 3;
  if (t < 2 / 3) return 2;
  return 1;
}

function divisionProgress(
  sr: number,
  min: number,
  nextMin: number,
  division: LeagueDivision | null,
): number {
  const width = nextMin - min;
  if (width <= 0) return 1;
  if (division == null) {
    return Math.min(1, Math.max(0, (sr - min) / width));
  }
  const slice = width / 3;
  const divIndex = 3 - division;
  const divMin = min + divIndex * slice;
  return Math.min(1, Math.max(0, (sr - divMin) / slice));
}

function roman(d: LeagueDivision): string {
  return d === 1 ? "I" : d === 2 ? "II" : "III";
}

function buildRank(sr: number): LeagueRank {
  const clamped = clampSr(sr);
  const { id, label, min, nextMin } = tierAt(clamped);
  const division = divisionInBand(clamped, min, nextMin);
  return {
    tier: id,
    division,
    label: division != null ? `${label} ${roman(division)}` : label,
    shortLabel:
      division != null ? `${label.charAt(0)}${roman(division)}` : label.slice(0, 2),
    sr: Math.round(clamped),
    progress: divisionProgress(clamped, min, nextMin, division),
    placementLeft: 0,
  };
}

/** Cap displayed tier if series volume is not there yet. */
function applyVolumeGate(rank: LeagueRank, seriesCount: number): LeagueRank {
  const idx = TIER_BANDS.findIndex((b) => b.id === rank.tier);
  if (idx < 0) return rank;

  let allowedIdx = idx;
  while (allowedIdx > 0) {
    const tier = TIER_BANDS[allowedIdx]!;
    const need = TIER_MIN_SERIES[tier.id] ?? 0;
    if (seriesCount >= need) break;
    allowedIdx -= 1;
  }
  if (allowedIdx === idx) return rank;

  const nextMin = TIER_BANDS[allowedIdx + 1]?.min ?? SR_MAX;
  // Park at top of allowed band so progress still reads full until volume unlocks.
  const parkSr = nextMin - 1;
  return buildRank(parkSr);
}

export function rankFromSr(sr: number, seriesCount: number): LeagueRank {
  if (seriesCount < PLACEMENT_SERIES) {
    return {
      tier: "unranked",
      division: null,
      label: "Platzierung",
      shortLabel: "—",
      sr: 0,
      progress: seriesCount / PLACEMENT_SERIES,
      placementLeft: PLACEMENT_SERIES - seriesCount,
    };
  }
  return applyVolumeGate(buildRank(sr), seriesCount);
}

function avg(nums: number[]): number {
  if (nums.length === 0) return 0;
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

/**
 * Walk sessions oldest → newest and derive current league rank.
 * Empty / short history → Platzierung.
 */
export function leagueFromSessions(sessions: TrainingSessionSummary[]): LeagueRank {
  if (sessions.length === 0) {
    return rankFromSr(0, 0);
  }
  if (sessions.length < PLACEMENT_SERIES) {
    return rankFromSr(0, sessions.length);
  }

  const placement = sessions.slice(0, PLACEMENT_SERIES);
  let sr = pointsToSr(avg(placement.map((s) => s.punkteTotal)));
  for (const s of sessions.slice(PLACEMENT_SERIES)) {
    sr = applySerieToSr(sr, s.punkteTotal);
  }
  return rankFromSr(sr, sessions.length);
}

/** Group sessions by shooter filter key and compute each league. */
export function leagueMapFromSessions(
  sessions: TrainingSessionSummary[],
  keyOf: (s: { personId?: string | null; shooterName: string }) => string,
): Map<string, LeagueRank> {
  const byKey = new Map<string, TrainingSessionSummary[]>();
  for (const s of sessions) {
    const key = keyOf(s);
    const list = byKey.get(key);
    if (list) list.push(s);
    else byKey.set(key, [s]);
  }
  const out = new Map<string, LeagueRank>();
  for (const [key, list] of byKey) {
    out.set(key, leagueFromSessions(list));
  }
  return out;
}

export const LEAGUE_TIER_ORDER: LeagueTierId[] = [
  "unranked",
  "bronze",
  "silver",
  "gold",
  "platinum",
  "diamond",
  "master",
  "grandmaster",
  "champion",
];
