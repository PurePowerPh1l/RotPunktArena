/**
 * Contract checks for appearance resolution.
 * Run: node --experimental-strip-types apps/desktop/src/lib/appearanceLogic.selftest.ts
 */
import { resolveEffectiveColorScheme } from "./appearanceLogic.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

assert(resolveEffectiveColorScheme("light", true) === "light", "light wins");
assert(resolveEffectiveColorScheme("dark", false) === "dark", "dark wins");
assert(
  resolveEffectiveColorScheme("system", true) === "dark",
  "system + OS dark",
);
assert(
  resolveEffectiveColorScheme("system", false) === "light",
  "system + OS light",
);

console.log("appearanceLogic.selftest: ok");
