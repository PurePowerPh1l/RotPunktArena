import type { TrainingSessionSummary, EntryResultSummary } from "@rotpunktarena/domain";
import { leagueFromSessions } from "./league.ts";
import { computeTrainingStats } from "./stats.ts";
import { computeTransfer } from "./transfer.ts";

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
const series = (shots: number, index: number) => ({ id: `${shots}-${index}`, shotCount: shots,
  punkteTotal: shots * 10, teilerAvg: 20, teilerSum: shots * 20, teilerBest: 10, shooterName: "Test",
  startedAt: "2026-10-01", endedAt: "2026-10-01" } as TrainingSessionSummary);
let expectedRank: string | undefined;
for (const shots of [5, 10, 20, 30]) {
  const history = Array.from({ length: 9 }, (_, i) => series(shots, i));
  const rank = JSON.stringify(leagueFromSessions(history));
  expectedRank ??= rank;
  assert(rank === expectedRank, `equal performance must give equal rank for ${shots} shots`);
  const stats = computeTrainingStats(history);
  assert(stats.avgSeriePunkte === 100 && stats.bestSerie === 100 && stats.trendPunkte === 0, "normalised comparisons");
  assert(stats.shotCount === 9 * shots && stats.avgPunkteProSchuss === 10, "preserve actual volume");
  const transfer = computeTransfer(history, [{ shotCount: 30, punkteTotal: 300 } as EntryResultSummary]);
  assert(transfer?.delta === 0 && transfer.competitionAvg === 100, "mixed competition length");
}
const mixed = [5, 10, 20, 30].map(series);
const stats = computeTrainingStats(mixed);
assert(stats.trendPunkte === 0 && stats.avgSeriePunkte === 100, "mixed lengths must not create improvement");
const weighted = computeTrainingStats([{ ...series(5, 0), teilerAvg: 10 }, { ...series(30, 1), teilerAvg: 20 }]);
assert(Math.abs(weighted.avgTeiler - 650 / 35) < 1e-9, "weighted teiler average");
console.log("training comparison tests OK");
