/**
 * Contract checks for arena preference helpers.
 * Run: node --experimental-strip-types apps/desktop/src/lib/arenaPrefsLogic.selftest.ts
 */
import {
  effectiveScoreDisplay,
  hitFeedbackShowsFlash,
  scoreDisplayOverrideForMode,
  shouldPersistScoreDisplay,
  targetFitMultiplier,
} from "./arenaPrefsLogic.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

assert(
  effectiveScoreDisplay({
    mode: "training",
    pref: "teiler",
  }) === "teiler",
  "training uses pref",
);

assert(
  effectiveScoreDisplay({
    mode: "competition",
    competitionScoringMode: "teiler",
    pref: "punkte",
  }) === "teiler",
  "competition teiler wins over pref",
);

assert(
  effectiveScoreDisplay({
    mode: "competition",
    competitionScoringMode: "ringe",
    pref: "teiler",
  }) === "punkte",
  "competition ringe → punkte; pref not written",
);

assert(targetFitMultiplier("calm") < targetFitMultiplier("auto"), "calm < auto");
assert(
  targetFitMultiplier("auto") < targetFitMultiplier("aggressive"),
  "auto < aggressive",
);
assert(targetFitMultiplier("auto") === 1, "auto is 1");

assert(hitFeedbackShowsFlash("normal"), "normal shows flash");
assert(hitFeedbackShowsFlash("reduced"), "reduced shows flash");
assert(!hitFeedbackShowsFlash("minimal"), "minimal hides flash");

assert(!shouldPersistScoreDisplay(false), "no writeback when remember off");
assert(shouldPersistScoreDisplay(true), "writeback when remember on");
assert(
  scoreDisplayOverrideForMode("competition", "teiler") === null,
  "competition clears override",
);

console.log("arenaPrefsLogic.selftest: ok");
