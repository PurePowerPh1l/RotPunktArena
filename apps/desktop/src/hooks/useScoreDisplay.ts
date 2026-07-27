import { useMemo } from "react";
import type { UiShot } from "@reddot/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";

/** Points/Teiler primary/secondary + series Σ from Rust fields (no local reduce). */
export function useScoreDisplay(
  displayMode: ScoreDisplayMode,
  last: UiShot | null | undefined,
  seriesTotal: number | null | undefined,
  seriesTeilerTotal: number | null | undefined,
) {
  return useMemo(() => {
    const primary = last
      ? displayMode === "teiler"
        ? last.distanceDisplay
        : last.valueDisplay
      : null;
    const secondary = last
      ? displayMode === "teiler"
        ? last.valueDisplay
        : last.distanceDisplay
      : null;
    const seriesPrimary =
      displayMode === "teiler" ? (seriesTeilerTotal ?? 0) : (seriesTotal ?? 0);
    const primaryUnit = displayMode === "teiler" ? "Teiler" : "Punkte";
    const secondaryUnit = displayMode === "teiler" ? "Punkte" : "Teiler";
    const seriesUnit = displayMode === "teiler" ? "Σ Teiler" : "Σ Punkte";
    return {
      primary,
      secondary,
      seriesPrimary,
      primaryUnit,
      secondaryUnit,
      seriesUnit,
    };
  }, [displayMode, last, seriesTotal, seriesTeilerTotal]);
}

export function shotRowValues(shot: UiShot, displayMode: ScoreDisplayMode) {
  const primary = displayMode === "teiler" ? shot.distanceDisplay : shot.valueDisplay;
  const secondary = displayMode === "teiler" ? shot.valueDisplay : shot.distanceDisplay;
  const sigma = displayMode === "teiler" ? shot.seriesTeilerTotal : shot.seriesTotal;
  return { primary, secondary, sigma };
}

/**
 * Best single shot for the active display metric.
 * Punkte: highest ring value; Teiler: lowest distance (lower is better).
 * Tie-break: earliest shotIndex.
 */
export function bestShotOf(
  shots: UiShot[],
  displayMode: ScoreDisplayMode,
): UiShot | null {
  if (shots.length === 0) return null;
  return shots.reduce((best, s) => {
    if (displayMode === "teiler") {
      if (s.distanceDisplay < best.distanceDisplay) return s;
      if (s.distanceDisplay > best.distanceDisplay) return best;
    } else {
      if (s.valueDisplay > best.valueDisplay) return s;
      if (s.valueDisplay < best.valueDisplay) return best;
    }
    return s.shotIndex < best.shotIndex ? s : best;
  });
}
