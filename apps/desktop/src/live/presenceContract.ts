/**
 * Live Presence — parallel phase contract (do not edit LiveStandView from phase agents).
 *
 * Ownership (exclusive writes):
 *   A Halo      → components/TargetFace.tsx + styles/live.css classes: .face-label-pill*
 *   B Moments   → hooks/useTrainingMoments.ts + components/live/MomentFlash.tsx
 *                 + styles/live.css: .moment-*
 *   C Streak    → components/live/StreakChip.tsx + styles/live.css: .streak-chip*
 *   D Audio     → hooks/useShotSound.ts only (extend playShot API)
 *   E Ceremony  → components/live/SeriesCeremony.tsx + styles/live.css: .series-ceremony*
 *
 * Integration (parent only): LiveStandView.tsx, LiveScoreColumn.tsx, App.tsx
 *
 * Mode gate: gamification (B/C/D-tiers/E) ONLY when mode === "training".
 * Competition: Halo labels + last/best highlight only.
 */
import type { UiShot } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";

/** Audio cue tier — training may pass rich tiers; competition keeps miss|hit. */
export type ShotSoundTier = "miss" | "hit" | "mid" | "ten" | "inner" | "best" | "seriesDone";

export type PlayShotOpts = {
  miss?: boolean;
  /** When set (training), pick graded cue. Ignored if muted. */
  tier?: ShotSoundTier;
};

/** Classify a shot for training micromoments / audio (UI heuristic, not domain rules). */
export function classifyShotMoment(
  shot: UiShot,
  displayMode: ScoreDisplayMode,
): "miss" | "mid" | "ten" | "inner" {
  if (shot.valueDisplay <= 0) return "miss";
  // Innenzehn heuristic: near-perfect teiler or ≥10.9 rings
  if (shot.valueDisplay >= 10.9 || (displayMode === "teiler" && shot.distanceDisplay <= 1.0)) {
    return "inner";
  }
  if (shot.valueDisplay >= 10) return "ten";
  return "mid";
}

/** Count trailing tens (valueDisplay >= 10) from the end of the series. */
export function tenStreak(shots: UiShot[]): number {
  let n = 0;
  for (let i = shots.length - 1; i >= 0; i--) {
    if ((shots[i]?.valueDisplay ?? 0) < 10) break;
    n += 1;
  }
  return n;
}
