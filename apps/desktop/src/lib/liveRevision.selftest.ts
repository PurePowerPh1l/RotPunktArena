const assert = { equal(actual: unknown, expected: unknown) { if (actual !== expected) throw new Error("live revision assertion failed"); } };
import { chooseLiveSnapshot } from "./liveRevision.ts";
import type { LiveState } from "@rotpunktarena/domain";
import contract from "../../../../fixtures/live-idle.json" with { type: "json" };
import invalidContracts from "../../../../fixtures/live-invalid.json" with { type: "json" };

// Exercise only ordering metadata; the full JSON contract has a shared fixture.
const state = (revision: number, id: string | null, phase: LiveState["phase"] = "match") =>
  ({ ...contract, revision, sessionId: id, session: id ? { id } : null, phase, shots: [] } as unknown as LiveState);
const scored = state(5, "first");
assert.equal(chooseLiveSnapshot(scored, state(4, "first", "probe")), scored);
assert.equal(chooseLiveSnapshot(scored, state(5, "first")), scored);
const restarted = state(8, "next");
assert.equal(chooseLiveSnapshot(scored, restarted), restarted); // gap = authoritative resync
assert.equal(chooseLiveSnapshot(restarted, state(7, "first", "closed")), restarted);
const ended = state(9, "next", "closed");
assert.equal(chooseLiveSnapshot(restarted, ended), ended);
const reset = state(10, null, "idle");
assert.equal(chooseLiveSnapshot(ended, reset), reset);
assert.equal(chooseLiveSnapshot(reset, { ...state(11, "x"), sessionId: "wrong" }), reset);
assert.equal(chooseLiveSnapshot(reset, state(NaN, "x")), reset);
const idle = contract as LiveState;
assert.equal(chooseLiveSnapshot(null, idle), idle);
assert.equal(idle.phase, "idle");
assert.equal(idle.sessionId, null);
assert.equal(idle.lastShot, null);
for (const test of invalidContracts) {
  assert.equal(chooseLiveSnapshot(idle, { ...idle, ...test.patch } as unknown as LiveState), idle);
}
console.log("live revision tests OK");
