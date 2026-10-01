import { readFileSync } from "node:fs";
import {
  CONTROL,
  RedDotStreamParser,
  buildSyntheticShotFrame,
  distanceDisplay,
  encodeDc1,
  encodeEnq,
  parseShotFrame,
  valueDisplay,
  DC1_CMD,
} from "./index";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

const frame = buildSyntheticShotFrame({
  valueAscii: "10.5",
  distanceAscii: "012.30",
  xAscii: "00123",
  yAscii: "-0045",
});

const shot = parseShotFrame(frame);
assert(shot.valueRaw === 105, `valueRaw ${shot.valueRaw}`);
assert(shot.distanceRaw === 1230, `distanceRaw ${shot.distanceRaw}`);
assert(shot.x === 123, `x ${shot.x}`);
assert(shot.y === -45, `y ${shot.y}`);
assert(valueDisplay(105, true) === 10.5, "value display tenths");
assert(valueDisplay(105, false) === 10, "value display full rings");
assert(distanceDisplay(1230) === 123, "distance display");

const parser = new RedDotStreamParser();
let events = parser.push(Uint8Array.of(CONTROL.NAK));
assert(events[0]?.type === "nak", "nak");

events = parser.push(encodeEnq()); // host bytes ignored if echoed — skip
events = parser.push(frame);
assert(events.some((e) => e.type === "shot"), "shot event");
const shotEv = events.find((e) => e.type === "shot");
assert(shotEv?.type === "shot" && shotEv.shot.valueRaw === 105, "shot value");

// Split frame across chunks
const p2 = new RedDotStreamParser();
events = p2.push(frame.subarray(0, 20));
assert(events.some((e) => e.type === "need_more"), "need_more");
events = p2.push(frame.subarray(20));
assert(events.some((e) => e.type === "shot"), "shot after split");

const getVars = encodeDc1(DC1_CMD.getVars);
assert(
  getVars[0] === CONTROL.DC1 && getVars[1] === 0x0f && getVars[2] === 0xb4,
  "getVars encoding",
);

const fixtures = JSON.parse(readFileSync(new URL("../../../fixtures/protocol.json", import.meta.url), "utf8"));
assert(fixtures.version === 1, "fixture version");
for (const test of fixtures.cases) {
  const [valueAscii, distanceAscii, xAscii, yAscii] = test.fields;
  const raw = buildSyntheticShotFrame({ valueAscii, distanceAscii, xAscii, yAscii });
  let actual: number[] | null = null;
  try {
    const parsed = parseShotFrame(raw);
    actual = [parsed.valueRaw, parsed.distanceRaw, parsed.x, parsed.y];
  } catch { /* Invalid cases must be rejected. */ }
  assert(JSON.stringify(actual) === JSON.stringify(test.expected), `shared fixture ${test.name}`);
  // A malformed frame must preserve diagnostics and allow the following valid frame.
  const stream = new RedDotStreamParser();
  const output = stream.push(Uint8Array.from([...raw, ...frame]));
  assert(output.length === 2 && output[1]?.type === "shot", `resync ${test.name}`);
  assert(output[0]?.type === (test.expected === null ? "parse_error" : "shot"), `stream ${test.name}`);
  if (output[0]?.type === "parse_error") {
    assert(output[0].raw.every((byte, index) => byte === raw[index]), "preserve invalid bytes");
  }
}
console.log("protocol tests OK");
