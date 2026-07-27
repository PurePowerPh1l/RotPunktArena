import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeCaptureText } from "../src/analyze.ts";

const here = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(here, "../../../docs/captures/synthetic-shot.hex");

describe("analyzeCaptureText", () => {
  it("dumps header and trailer regions from synthetic fixture", () => {
    const text = readFileSync(fixturePath, "utf8");
    const dumps = analyzeCaptureText(text);
    assert.equal(dumps.length, 1);
    const d = dumps[0]!;
    assert.equal(d.headerBytes.length, 31);
    assert.equal(d.trailerBytes.length, 4);
    assert.equal(d.valueRaw, 105);
    assert.equal(d.x, 123);
    assert.equal(d.y, -45);
    // Synthetic fixture pads header with spaces (0x20)
    assert.ok(d.headerHex.includes("20"));
  });
});
