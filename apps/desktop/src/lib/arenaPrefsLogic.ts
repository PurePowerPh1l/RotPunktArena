import type {
  HitFeedbackPref,
  ScoreDisplayPref,
  TargetFitPref,
} from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";

/**
 * Default display: Wettkampf folgt scoringMode, Training der Nutzer-Pref.
 * Live-Umschalten Punkte/Teiler ist in beiden Modi erlaubt (Session-Override
 * in der View); Pref wird nur im Training geschrieben.
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

/** Whether a training display toggle may write `scoreDisplay` into ui.prefs. */
export function shouldPersistScoreDisplay(rememberScoreDisplay: boolean): boolean {
  return rememberScoreDisplay;
}

/** Drop session view override when entering competition (use scoring default). */
export function scoreDisplayOverrideForMode(
  mode: "training" | "competition",
  trainingOverride: ScoreDisplayMode | null,
): ScoreDisplayMode | null {
  return mode === "competition" ? null : trainingOverride;
}

/** Whether MomentFlash should mount at all. */
export function hitFeedbackShowsFlash(feedback: HitFeedbackPref): boolean {
  return feedback !== "minimal";
}

export function hitFeedbackIntensityClass(
  feedback: HitFeedbackPref,
): string | undefined {
  if (feedback === "reduced") return "is-feedback-reduced";
  return undefined;
}
