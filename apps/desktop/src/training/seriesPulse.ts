import type { TrainingSessionSummary } from "@rotpunktarena/domain";
import { leagueFromSessions, type LeagueRank } from "./league";
import {
  computeTrainingStats,
  type TrainingStats,
  xpFromSessions,
} from "./stats";

/** Soft UI pulse after a saved training series (§10 gamification). */
export type SeriesPulse = {
  seriePunkte: number;
  xpGained: number;
  leveledUp: boolean;
  levelAfter: number;
  levelTitleAfter: string;
  levelProgressAfter: number;
  xpToNext: number;
  /** Null while still in placement (no SR yet). */
  srDelta: number | null;
  league: LeagueRank;
  leagueBefore: LeagueRank;
  tierChanged: boolean;
  placeHint: string | null;
  placeHintKind: "up" | "down" | "flat" | "none";
  /** Delta vs optional rival target (serie Punkte). */
  rivalDelta: number | null;
  rivalLabel: string | null;
};

export type TrainingProgressSnapshot = {
  stats: TrainingStats;
  league: LeagueRank;
  sessionCount: number;
};

const PLACE_WINDOW = 5;

/** API already returns oldest → newest; keep helper for safety. */
export function sessionsChronological(
  sessions: TrainingSessionSummary[],
): TrainingSessionSummary[] {
  if (sessions.length < 2) return sessions;
  const first = sessions[0]?.endedAt ?? "";
  const last = sessions[sessions.length - 1]?.endedAt ?? "";
  if (first <= last) return sessions;
  return [...sessions].reverse();
}

export function progressFromSessions(
  sessions: TrainingSessionSummary[],
): TrainingProgressSnapshot {
  const chrono = sessionsChronological(sessions);
  return {
    stats: computeTrainingStats(chrono),
    league: leagueFromSessions(chrono),
    sessionCount: chrono.length,
  };
}

export function placeHintForSerie(
  seriePunkte: number,
  prior: TrainingSessionSummary[],
): { text: string | null; kind: SeriesPulse["placeHintKind"] } {
  if (prior.length === 0) {
    return { text: "erste Serie", kind: "flat" };
  }
  const window = prior.slice(-PLACE_WINDOW);
  const avg =
    window.reduce((sum, s) => sum + s.punkteTotal, 0) / window.length;
  const delta = seriePunkte - avg;
  if (Math.abs(delta) < 0.5) {
    return { text: "im Rahmen deines Schnitts", kind: "flat" };
  }
  if (delta > 0) {
    return { text: "über deinem Schnitt", kind: "up" };
  }
  return { text: "unter deinem Schnitt", kind: "down" };
}

/**
 * Build pulse from history that already includes the just-saved series
 * as the newest entry. Optional rival is a target series total.
 */
export function computeSeriesPulse(
  sessionsIncludingLatest: TrainingSessionSummary[],
  rival?: { label: string; punkte: number } | null,
): SeriesPulse | null {
  const chrono = sessionsChronological(sessionsIncludingLatest);
  if (chrono.length === 0) return null;

  const latest = chrono[chrono.length - 1]!;
  const prior = chrono.slice(0, -1);
  const afterStats = computeTrainingStats(chrono);
  const beforeStats = computeTrainingStats(prior);
  const afterLeague = leagueFromSessions(chrono);
  const beforeLeague = leagueFromSessions(prior);

  const xpGained = Math.max(
    0,
    xpFromSessions([latest]),
  );
  const leveledUp = afterStats.level > beforeStats.level;
  const tierChanged =
    afterLeague.tier !== beforeLeague.tier ||
    afterLeague.division !== beforeLeague.division ||
    (beforeLeague.tier === "unranked" && afterLeague.tier !== "unranked");

  const inPlacement =
    afterLeague.tier === "unranked" || beforeLeague.tier === "unranked";
  const srDelta = inPlacement
    ? null
    : afterLeague.sr - beforeLeague.sr;

  const place = placeHintForSerie(latest.punkteTotal, prior);
  let placeHint = place.text;
  let placeHintKind = place.kind;
  let rivalDelta: number | null = null;
  let rivalLabel: string | null = null;

  if (rival && Number.isFinite(rival.punkte)) {
    rivalDelta = latest.punkteTotal - rival.punkte;
    rivalLabel = rival.label;
    if (Math.abs(rivalDelta) < 0.5) {
      placeHint = `gleichauf mit ${rival.label}`;
      placeHintKind = "flat";
    } else if (rivalDelta > 0) {
      placeHint = `+${rivalDelta.toLocaleString("de-DE", {
        maximumFractionDigits: 1,
      })} vor ${rival.label}`;
      placeHintKind = "up";
    } else {
      placeHint = `${rivalDelta.toLocaleString("de-DE", {
        maximumFractionDigits: 1,
      })} hinter ${rival.label}`;
      placeHintKind = "down";
    }
  }

  return {
    seriePunkte: latest.punkteTotal,
    xpGained,
    leveledUp,
    levelAfter: afterStats.level,
    levelTitleAfter: afterStats.levelTitle,
    levelProgressAfter: afterStats.levelProgress,
    xpToNext: afterStats.xpToNext,
    srDelta,
    league: afterLeague,
    leagueBefore: beforeLeague,
    tierChanged,
    placeHint,
    placeHintKind,
    rivalDelta,
    rivalLabel,
  };
}

/** Rough XP for the live series before history refresh (same formula as stats). */
export function xpPreviewForLiveSeries(
  punkteTotal: number,
  shotCount: number,
): number {
  return Math.max(0, punkteTotal) + Math.max(0, shotCount) * 1.5;
}

/** Pick own last completed series as a soft rival target (Eigen-Ghost). */
export function pickEigenRival(
  sessions: TrainingSessionSummary[],
): { label: string; punkte: number } | null {
  const chrono = sessionsChronological(sessions);
  if (chrono.length === 0) return null;
  const last = chrono[chrono.length - 1]!;
  return {
    label: "deiner letzten Serie",
    punkte: last.punkteTotal,
  };
}
