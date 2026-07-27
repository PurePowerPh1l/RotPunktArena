/**
 * Contract checks for resolveStartView + rememberLastView merge.
 * Run: node --experimental-strip-types apps/desktop/src/lib/uiPrefsLogic.selftest.ts
 */
import { UI_PREFS_LOAD_PLACEHOLDER, type UiPrefs } from "@rotpunktarena/domain";
import { mergeUiPrefsPatch, resolveStartView } from "./uiPrefsLogic.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

const base: UiPrefs = { ...UI_PREFS_LOAD_PLACEHOLDER };

assert(resolveStartView(base) === "live", "default → startView live");

assert(
  resolveStartView({
    ...base,
    startView: "bureau",
    rememberLastView: false,
    lastView: "history",
  }) === "bureau",
  "remember off ignores lastView",
);

assert(
  resolveStartView({
    ...base,
    startView: "bureau",
    rememberLastView: true,
    lastView: "history",
  }) === "history",
  "remember on uses lastView",
);

assert(
  resolveStartView({
    ...base,
    startView: "bureau",
    rememberLastView: true,
    lastView: null,
  }) === "bureau",
  "remember on + null lastView → startView",
);

const cleared = mergeUiPrefsPatch(
  { ...base, rememberLastView: true, lastView: "history" },
  { rememberLastView: false },
);
assert(cleared.rememberLastView === false, "remember off");
assert(cleared.lastView === null, "lastView cleared when remember off");

console.log("uiPrefsLogic.selftest: ok");
