/**
 * Geometry selftest — no DOM.
 * Run: npm run test:geometry --workspace=@reddot/desktop
 */
import {
  BLACK_AIM_RADIUS_SVG,
  DEVICE_RADIUS_AT_RING_1,
  MAP_RADIUS,
  NUMBERED_SCORING_RINGS,
  RING_STEP_SVG,
  VIEW_HALF,
  scoringRingBandMidSvg,
  scoringRingLabelSpecs,
  scoringRingOuterSvg,
  scoringRingSpecs,
  shotCoordsToSvg,
  svgRadiusFromDeviceHypot,
} from "./targetFaceGeometry.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

function approx(actual: number, expected: number, eps = 0.001): void {
  assert(
    Math.abs(actual - expected) <= eps,
    `expected ${expected} ±${eps}, got ${actual}`,
  );
}

const origin = shotCoordsToSvg(0, 0);
assert(origin.cx === 50 && origin.cy === 50, "origin → (50,50)");

const up = shotCoordsToSvg(0, DEVICE_RADIUS_AT_RING_1);
approx(up.cx, 50);
approx(up.cy, 8); // 50 - 42

const right = shotCoordsToSvg(DEVICE_RADIUS_AT_RING_1, 0);
approx(right.cx, 92); // 50 + 42
approx(right.cy, 50);

approx(svgRadiusFromDeviceHypot(70), 1.176);
approx(svgRadiusFromDeviceHypot(508.6), 8.54448, 0.002);
approx(svgRadiusFromDeviceHypot(522.1), 8.77128, 0.002);

approx(svgRadiusFromDeviceHypot(DEVICE_RADIUS_AT_RING_1), MAP_RADIUS);
approx(svgRadiusFromDeviceHypot(2750), 46.2);

const zeroEdge = svgRadiusFromDeviceHypot(2750);
assert(zeroEdge < VIEW_HALF, `0.0 edge ${zeroEdge} must stay inside paper ${VIEW_HALF}`);

// Hardware samples as stored x/y along +x (same hypot)
approx(Math.hypot(shotCoordsToSvg(508.6, 0).cx - 50, shotCoordsToSvg(508.6, 0).cy - 50), 8.54448, 0.002);

approx(RING_STEP_SVG, 4.2);
approx(scoringRingOuterSvg(1), 42);
approx(scoringRingOuterSvg(4), 29.4);
approx(scoringRingOuterSvg(9), 8.4);
approx(scoringRingOuterSvg(10), 4.2);
approx(BLACK_AIM_RADIUS_SVG, scoringRingOuterSvg(4));
approx(scoringRingOuterSvg(9) - scoringRingOuterSvg(10), RING_STEP_SVG);
approx(scoringRingOuterSvg(1) - scoringRingOuterSvg(2), RING_STEP_SVG);

const specs = scoringRingSpecs();
assert(specs.length === 10, "ten scoring rings");
assert(specs[0]!.ring === 1 && !specs[0]!.onBlack, "ring 1 on paper");
assert(specs[3]!.ring === 4 && specs[3]!.onBlack, "ring 4 on black");
assert(specs[9]!.ring === 10 && specs[9]!.onBlack, "ring 10 on black");

let threw = false;
try {
  scoringRingOuterSvg(0);
} catch {
  threw = true;
}
assert(threw, "ring 0 rejected");

approx(scoringRingBandMidSvg(1), (42 + 37.8) / 2);
approx(scoringRingBandMidSvg(4), (29.4 + 25.2) / 2);
approx(scoringRingBandMidSvg(10), 2.1);

const labels = scoringRingLabelSpecs();
assert(labels.length === NUMBERED_SCORING_RINGS.length, "eight numbered rings");
assert(labels.every((l) => l.positions.length === 4), "four axes each");
assert(!labels.some((l) => l.ring === 9 || l.ring === 10), "9/10 unnumbered");
assert(labels[0]!.positions[0]!.axis === "n" && labels[0]!.positions[0]!.cy < 50, "N above center");
assert(labels[3]!.onBlack, "ring 4 labels on black");
assert(!labels[0]!.onBlack, "ring 1 labels on paper");

console.log("targetFaceGeometry selftest OK");
