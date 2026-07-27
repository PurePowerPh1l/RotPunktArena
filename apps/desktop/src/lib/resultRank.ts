import type { EntryResultSummary, TeamResultSummary } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";

/** Opaque rank from Rust — UI may order by this, never recompute from scores. */
export function entryRank(
  r: EntryResultSummary,
  sortMode: ScoreDisplayMode,
): number | null {
  const rank = sortMode === "teiler" ? r.rankTeiler : r.rankPunkte;
  return rank ?? null;
}

export function teamRank(
  t: TeamResultSummary,
  sortMode: ScoreDisplayMode,
): number | null {
  const rank = sortMode === "teiler" ? t.rankTeiler : t.rankPunkte;
  return rank ?? null;
}

export function compareByEntryRank(
  a: EntryResultSummary,
  b: EntryResultSummary,
  sortMode: ScoreDisplayMode,
): number {
  const ra = entryRank(a, sortMode);
  const rb = entryRank(b, sortMode);
  if (ra == null && rb == null) return a.startOrder - b.startOrder;
  if (ra == null) return 1;
  if (rb == null) return -1;
  return ra - rb;
}

export function compareByTeamRank(
  a: TeamResultSummary,
  b: TeamResultSummary,
  sortMode: ScoreDisplayMode,
): number {
  const ra = teamRank(a, sortMode);
  const rb = teamRank(b, sortMode);
  if (ra == null && rb == null) return a.sortOrder - b.sortOrder;
  if (ra == null) return 1;
  if (rb == null) return -1;
  return ra - rb;
}
