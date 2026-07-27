import type {
  HitFeedbackPref,
  ScoreDisplayPref,
  TargetFitPref,
} from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";

/**
 * Wettkampf schlägt Nutzer-Pref, überschreibt sie aber nie.
 * Competition scoringMode `ringe` maps to display `punkte`.
 */
export function effectiveScoreDisplay(opts: {
  mode: "training" | "competition";
  competitionScoringMode?: "ringe" | "teiler" | null;
  pref: ScoreDisplayPref;
}): ScoreDisplayMode {
  if (opts.mode === "competition") {
    return opts.competitionScoringMode === "teiler" ? "teiler" : "punkte";
  }
  return opts.pref;
}

/** Fit multiplier around TargetFace `fitFaceScale` (calm < auto < aggressive). */
export function targetFitMultiplier(fit: TargetFitPref): number {
  switch (fit) {
    case "calm":
      return 0.92;
    case "aggressive":
      return 1.12;
    case "auto":
    default:
      return 1;
  }
}

/** Whether MomentFlash / ceremony intensity should show. */
export function hitFeedbackShowsFlash(feedback: HitFeedbackPref): boolean {
  return feedback !== "minimal";
}

export function hitFeedbackIntensityClass(
  feedback: HitFeedbackPref,
): string | undefined {
  if (feedback === "reduced") return "is-feedback-reduced";
  return undefined;
}
