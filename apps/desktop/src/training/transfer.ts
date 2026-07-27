import type { EntryResultSummary, TrainingSessionSummary } from "@reddot/domain";
import { fmtStat } from "./stats";

export type TransferSnapshot = {
  trainingAvg: number;
  trainingCount: number;
  competitionAvg: number;
  competitionCount: number;
  delta: number;
  hint: string;
  kind: "up" | "down" | "flat" | "none";
};

function avg(nums: number[]): number {
  if (nums.length === 0) return 0;
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

/** Compare training series averages to competition best-series results. */
export function computeTransfer(
  training: TrainingSessionSummary[],
  competitionBests: EntryResultSummary[],
): TransferSnapshot | null {
  const trainPts = training
    .filter((s) => s.shotCount > 0)
    .map((s) => s.punkteTotal);
  const compPts = competitionBests
    .filter((r) => r.shotCount > 0)
    .map((r) => r.punkteTotal);

  if (trainPts.length === 0 && compPts.length === 0) return null;

  const trainingAvg = trainPts.length > 0 ? avg(trainPts) : 0;
  const competitionAvg = compPts.length > 0 ? avg(compPts) : 0;

  if (trainPts.length === 0 || compPts.length === 0) {
    return {
      trainingAvg,
      trainingCount: trainPts.length,
      competitionAvg,
      competitionCount: compPts.length,
      delta: 0,
      hint:
        trainPts.length === 0
          ? "Noch keine Trainings-Serien im Fenster"
          : "Noch keine Wettkampf-Ergebnisse",
      kind: "none",
    };
  }

  const delta = trainingAvg - competitionAvg;
  let kind: TransferSnapshot["kind"] = "flat";
  let hint = "Training ≈ Wettkampf";
  if (Math.abs(delta) < 0.8) {
    kind = "flat";
    hint = "Training kommt im Match an";
  } else if (delta > 0) {
    kind = "up";
    hint = "Training stärker als Match — Potenzial im Wettkampf";
  } else {
    kind = "down";
    hint = "Match über Training — Form hält unter Druck";
  }

  return {
    trainingAvg,
    trainingCount: trainPts.length,
    competitionAvg,
    competitionCount: compPts.length,
    delta,
    hint,
    kind,
  };
}

export function formatTransferDelta(delta: number): string {
  if (!Number.isFinite(delta) || Math.abs(delta) < 0.05) return "±0";
  const sign = delta > 0 ? "+" : "";
  return `${sign}${fmtStat(delta)}`;
}
