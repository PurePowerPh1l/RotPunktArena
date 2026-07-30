/**
 * Drift guard: FE placeholder must match Rust UiPrefs::default() JSON shape.
 * Run: node --experimental-strip-types packages/domain/src/settings.selftest.ts
 */
import { UI_PREFS_LOAD_PLACEHOLDER, type UiPrefs } from "./index.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

const expected: UiPrefs = {
  startView: "live",
  rememberLastView: false,
  lastView: null,
  compactUi: false,
  largeText: false,
  extraLargeUi: false,
  colorScheme: "system",
  reducedMotion: false,
  scoreDisplay: "punkte",
  rememberScoreDisplay: false,
  hitFeedback: "normal",
  targetFit: "auto",
  trainingSeriesShots: 10,
};

assert(
  JSON.stringify(UI_PREFS_LOAD_PLACEHOLDER) === JSON.stringify(expected),
  "UI_PREFS_LOAD_PLACEHOLDER drifted from expected Rust defaults",
);

const keys = Object.keys(UI_PREFS_LOAD_PLACEHOLDER).sort();
assert(
  keys.join(",") ===
    [
      "colorScheme",
      "compactUi",
      "extraLargeUi",
      "hitFeedback",
      "largeText",
      "lastView",
      "reducedMotion",
      "rememberLastView",
      "rememberScoreDisplay",
      "scoreDisplay",
      "startView",
      "targetFit",
      "trainingSeriesShots",
    ].join(","),
  `unexpected keys: ${keys.join(",")}`,
);

console.log("settings.selftest: ok");
