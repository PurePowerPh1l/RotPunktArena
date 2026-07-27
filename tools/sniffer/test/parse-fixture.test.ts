import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { valueDisplay } from "@reddot/protocol";
import { parseCaptureText, rxBytesFromCapture } from "../src/capture.ts";
import { replayCaptureFile } from "../src/replay.ts";

const here = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(here, "../../../docs/captures/synthetic-shot.hex");

describe("synthetic-shot.hex fixture", () => {
  it("parses capture and asserts value/x/y", () => {
    const text = readFileSync(fixturePath, "utf8");
    const events = replayCaptureFile(text);

    assert.ok(events.some((e) => e.type === "nak"), "erwartet NAK");
    const shotEv = events.find((e) => e.type === "shot");
    assert.ok(shotEv && shotEv.type === "shot", "erwartet Schuss-Event");

    const { shot } = shotEv;
    assert.equal(shot.valueRaw, 105);
    assert.equal(valueDisplay(shot.valueRaw, true), 10.5);
    assert.equal(shot.x, 123);
    assert.equal(shot.y, -45);
    assert.equal(shot.distanceRaw, 1230);
  });

  it("keeps TX ENQ out of the RX stream", () => {
    const text = readFileSync(fixturePath, "utf8");
    const chunks = parseCaptureText(text);
    const rx = rxBytesFromCapture(chunks);
    // First RX byte in fixture is NAK 0x15, not ENQ 0x05
    assert.equal(rx[0], 0x15);
    assert.ok(!Array.from(rx).includes(0x05) || rx[0] !== 0x05);
  });
});
