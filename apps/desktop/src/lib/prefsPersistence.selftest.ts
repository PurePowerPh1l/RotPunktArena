/**
 * Save-queue + arena display persistence contracts for P5.
 * Run: node --experimental-strip-types apps/desktop/src/lib/prefsPersistence.selftest.ts
 */
import { UI_PREFS_LOAD_PLACEHOLDER, type UiPrefs } from "@rotpunktarena/domain";
import {
  effectiveScoreDisplay,
  hitFeedbackShowsFlash,
  scoreDisplayOverrideForMode,
  shouldPersistScoreDisplay,
  targetFitMultiplier,
} from "./arenaPrefsLogic.ts";
import {
  emptySaveQueue,
  onSaveFailed,
  onSaveFinished,
  onSaveRequested,
} from "./prefsSaveQueue.ts";
import { mergeUiPrefsPatch, resolveStartView } from "./uiPrefsLogic.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

const base: UiPrefs = { ...UI_PREFS_LOAD_PLACEHOLDER };

// --- Persistenz / Restart (DTO round-trip shape) ---
assert(resolveStartView(base) === "live", "fresh defaults start live");
const remembered = mergeUiPrefsPatch(base, {
  rememberLastView: true,
  lastView: "history",
  startView: "bureau",
});
assert(resolveStartView(remembered) === "history", "restart uses lastView");

// --- rememberLastView off clears lastView in written DTO ---
const cleared = mergeUiPrefsPatch(remembered, { rememberLastView: false });
assert(cleared.lastView === null, "lastView null in DTO when remember off");

// --- Save serialisierung: schneller Toggle behält nur letzten Wunsch ---
let q = emptySaveQueue<UiPrefs>();
let r = onSaveRequested(q, { ...base, compactUi: true });
assert(r.start?.compactUi === true, "first toggle starts save");
q = r.state;
r = onSaveRequested(q, { ...base, compactUi: false });
assert(r.start === null, "second toggle does not start parallel save");
assert(r.state.pending?.compactUi === false, "pending is last");
q = r.state;
r = onSaveRequested(q, { ...base, compactUi: true, largeText: true });
assert(r.state.pending?.compactUi === true, "pending replaced by newest");
assert(r.state.pending?.largeText === true, "newest full DTO kept");
q = r.state;
const done = onSaveFinished(q);
assert(done.continueWith?.largeText === true, "after save continue with last pending");
assert(done.continueWith?.compactUi === true, "continued DTO is newest");
const idle = onSaveFinished(done.state);
assert(idle.continueWith === null && idle.state.inflight === false, "queue idle");
assert(onSaveFailed().inflight === false && onSaveFailed().pending === null, "fail clears");

// --- Competition override; Pref unberührt ---
assert(
  effectiveScoreDisplay({
    mode: "competition",
    competitionScoringMode: "teiler",
    pref: "punkte",
  }) === "teiler",
  "competition overrides display",
);
assert(
  effectiveScoreDisplay({
    mode: "training",
    competitionScoringMode: "teiler",
    pref: "punkte",
  }) === "punkte",
  "training uses pref, not competition scoring",
);

// --- rememberScoreDisplay writeback gate ---
assert(!shouldPersistScoreDisplay(false), "remember=false → no pref write");
assert(shouldPersistScoreDisplay(true), "remember=true → allow pref write");

// --- Training override cleared when entering competition ---
assert(
  scoreDisplayOverrideForMode("competition", "teiler") === null,
  "competition clears training override",
);
assert(
  scoreDisplayOverrideForMode("training", "teiler") === "teiler",
  "training keeps session override",
);

// --- hitFeedback minimal / fit only scaling ---
assert(!hitFeedbackShowsFlash("minimal"), "minimal → no flash mount");
assert(targetFitMultiplier("auto") === 1, "fit auto is identity scale");
assert(
  targetFitMultiplier("calm") !== targetFitMultiplier("aggressive"),
  "fit extremes differ",
);

console.log("prefsPersistence.selftest: ok");
