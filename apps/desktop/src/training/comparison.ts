/** Display comparison basis: points per shot projected onto ten shots. */
export function tenShotPoints(series: { punkteTotal: number; shotCount: number }): number {
  return series.shotCount > 0 ? series.punkteTotal * 10 / series.shotCount : 0;
}

/** Shared bounded history scope for Arena and statistics, never a lifetime total. */
export const TRAINING_HISTORY_WINDOW = 200;
