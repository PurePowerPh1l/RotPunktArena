/**
 * Run: node --experimental-strip-types --test tools/prefer-live-simulator.test.mts
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  hardwareLiveStartBlocked,
  resolveLiveExplicitSimulator,
  resolveLiveHardwareStart,
  serialAvailabilityFromLive,
} from "../apps/desktop/src/views/live/preferLiveSimulator.ts";

describe("serialAvailabilityFromLive", () => {
  it("maps true / false / undefined", () => {
    assert.equal(serialAvailabilityFromLive(true), "available");
    assert.equal(serialAvailabilityFromLive(false), "unavailable");
    assert.equal(serialAvailabilityFromLive(undefined), "unknown");
  });
});

describe("resolveLiveHardwareStart", () => {
  it("available + link → hardware", () => {
    assert.deepEqual(
      resolveLiveHardwareStart({ serial: "available", linkReady: true }),
      { action: "start", useSimulator: false },
    );
  });

  it("available + !link → blocked/link_required", () => {
    assert.deepEqual(
      resolveLiveHardwareStart({ serial: "available", linkReady: false }),
      { action: "blocked", reason: "link_required" },
    );
  });

  it("unavailable → simulator", () => {
    assert.deepEqual(
      resolveLiveHardwareStart({ serial: "unavailable", linkReady: false }),
      { action: "start", useSimulator: true },
    );
  });

  it("unavailable + linkReady → still simulator", () => {
    assert.deepEqual(
      resolveLiveHardwareStart({ serial: "unavailable", linkReady: true }),
      { action: "start", useSimulator: true },
    );
  });

  it("unknown → blocked/state_unavailable", () => {
    assert.deepEqual(
      resolveLiveHardwareStart({ serial: "unknown", linkReady: true }),
      { action: "blocked", reason: "state_unavailable" },
    );
  });

  it("unknown never yields start/useSimulator=true", () => {
    for (const linkReady of [true, false]) {
      const d = resolveLiveHardwareStart({ serial: "unknown", linkReady });
      assert.equal(d.action, "blocked");
      assert.notEqual(
        d.action === "start" && d.useSimulator === true,
        true,
      );
    }
  });
});

describe("resolveLiveExplicitSimulator", () => {
  it("always start / useSimulator=true", () => {
    assert.deepEqual(resolveLiveExplicitSimulator(), {
      action: "start",
      useSimulator: true,
    });
  });
});

describe("hardwareLiveStartBlocked", () => {
  it("matches hardware resolver blocked", () => {
    assert.equal(
      hardwareLiveStartBlocked({ serial: "available", linkReady: true }),
      false,
    );
    assert.equal(
      hardwareLiveStartBlocked({ serial: "available", linkReady: false }),
      true,
    );
    assert.equal(
      hardwareLiveStartBlocked({ serial: "unavailable", linkReady: false }),
      false,
    );
    assert.equal(
      hardwareLiveStartBlocked({ serial: "unknown", linkReady: true }),
      true,
    );
  });
});
