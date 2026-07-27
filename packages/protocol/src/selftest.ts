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

console.log("protocol tests OK");
