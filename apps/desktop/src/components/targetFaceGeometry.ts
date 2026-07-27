/**
 * Device → SVG face geometry (Slice A).
 * Hardware: hypot(x,y) === distanceDisplay; outer 1-ring ≈ 2500 device units.
 * Display score stays on the frame value field — never recompute from x/y.
 */

/** Device units at the outer 1-ring (maps to MAP_RADIUS). */
export const DEVICE_RADIUS_AT_RING_1 = 2500;

/** SVG units from center for DEVICE_RADIUS_AT_RING_1 (outer 1-ring). */
export const MAP_RADIUS = 42;

/** One full scoring ring in device units (10 rings → DEVICE_RADIUS_AT_RING_1). */
export const RING_STEP_DEVICE = 250;

/** One full scoring ring in SVG units (= MAP_RADIUS / 10). */
export const RING_STEP_SVG = MAP_RADIUS / 10;

/** Usable half of the 100×100 viewBox (paper / fit margin). */
export const VIEW_HALF = 48;

/**
 * Black aiming mark: center through outer edge of ring 4
 * (= scoringRingOuterSvg(4)).
 */
export const BLACK_AIM_RADIUS_SVG = (11 - 4) * RING_STEP_SVG;

/** Extra space around the farthest impact so labels/dots aren't clipped. */
const FIT_PAD = 4;

export type SvgPoint = { cx: number; cy: number };

export type ScoringRingSpec = {
  ring: number;
  outerSvg: number;
  /** Outer edge on/inside the black aiming mark (rings 4–10). */
  onBlack: boolean;
};

/** Outer SVG radius of integer scoring ring 1…10 (1 = outermost). */
export function scoringRingOuterSvg(ring: number): number {
  if (!Number.isInteger(ring) || ring < 1 || ring > 10) {
    throw new Error(`scoring ring must be integer 1..10, got ${ring}`);
  }
  return (11 - ring) * RING_STEP_SVG;
}

/** Rings 1…10 with radii shared by Live SVG and print renderers. */
export function scoringRingSpecs(): ScoringRingSpec[] {
  const rings: ScoringRingSpec[] = [];
  for (let ring = 1; ring <= 10; ring += 1) {
    rings.push({
      ring,
      outerSvg: scoringRingOuterSvg(ring),
      onBlack: ring >= 4,
    });
  }
  return rings;
}

/**
 * Mid-radius of scoring band `ring` (between outer edge of ring and next inner).
 * Ring 10 uses half of its outer radius (no inner integer ring).
 */
export function scoringRingBandMidSvg(ring: number): number {
  const outer = scoringRingOuterSvg(ring);
  const inner = ring >= 10 ? 0 : scoringRingOuterSvg(ring + 1);
  return (outer + inner) / 2;
}

export type ScoringRingLabelAxis = "n" | "e" | "s" | "w";

export type ScoringRingLabelPos = {
  cx: number;
  cy: number;
  axis: ScoringRingLabelAxis;
};

export type ScoringRingLabelSpec = {
  ring: number;
  onBlack: boolean;
  positions: ScoringRingLabelPos[];
};

/** Classic face numbers: rings 1–8 on N/E/S/W; 9 and 10 stay unnumbered. */
export const NUMBERED_SCORING_RINGS = [1, 2, 3, 4, 5, 6, 7, 8] as const;

/** Warm paper / black fills shared by Live gradient base and print. */
export const FACE_PAPER_FILL = "#e8dcc8";
export const FACE_BLACK_FILL = "#111110";

/** SVG font size for ring numbers in the 100×100 viewBox. */
export const RING_LABEL_FONT_SIZE_SVG = 2.9;

/** Four-axis label placements for numbered scoring rings. */
export function scoringRingLabelSpecs(
  rings: readonly number[] = NUMBERED_SCORING_RINGS,
): ScoringRingLabelSpec[] {
  return rings.map((ring) => {
    const r = scoringRingBandMidSvg(ring);
    return {
      ring,
      onBlack: ring >= 4,
      positions: [
        { cx: 50, cy: 50 - r, axis: "n" },
        { cx: 50 + r, cy: 50, axis: "e" },
        { cx: 50, cy: 50 + r, axis: "s" },
        { cx: 50 - r, cy: 50, axis: "w" },
      ],
    };
  });
}

/**
 * Static scoring-face chrome for print SVGs (paper, black, rings, numbers).
 * Shot marks are drawn by the caller on top.
 */
export function scoringFaceChromeSvg(): string {
  const rings = scoringRingSpecs()
    .map((spec) => {
      const stroke = spec.onBlack ? "rgba(245,240,230,0.55)" : "#3a3a36";
      return `<circle cx="50" cy="50" r="${spec.outerSvg}" fill="none" stroke="${stroke}" stroke-width="0.35"/>`;
    })
    .join("");
  const labels = scoringRingLabelSpecs()
    .flatMap((spec) =>
      spec.positions.map((p) => {
        const fill = spec.onBlack ? "rgba(245,240,230,0.92)" : "#2a241c";
        const stroke = spec.onBlack
          ? "rgba(0,0,0,0.55)"
          : "rgba(232,220,200,0.85)";
        return (
          `<text x="${p.cx}" y="${p.cy}" text-anchor="middle" dominant-baseline="central"` +
          ` font-size="${RING_LABEL_FONT_SIZE_SVG}" font-weight="700"` +
          ` font-family="Barlow Condensed, Arial Narrow, sans-serif"` +
          ` fill="${fill}" stroke="${stroke}" stroke-width="0.35"` +
          ` paint-order="stroke fill" style="pointer-events:none">${spec.ring}</text>`
        );
      }),
    )
    .join("");
  return (
    `<circle cx="50" cy="50" r="${VIEW_HALF}" fill="${FACE_PAPER_FILL}" stroke="#2a2a28" stroke-width="0.7"/>` +
    `<circle cx="50" cy="50" r="${BLACK_AIM_RADIUS_SVG}" fill="${FACE_BLACK_FILL}"/>` +
    rings +
    labels
  );
}

/** Map shot cartesian coords (device units) into SVG viewBox center (50,50). */
export function shotCoordsToSvg(x: number, y: number): SvgPoint {
  return {
    cx: 50 + (x / DEVICE_RADIUS_AT_RING_1) * MAP_RADIUS,
    cy: 50 - (y / DEVICE_RADIUS_AT_RING_1) * MAP_RADIUS,
  };
}

/** Radial distance from face center in SVG units for a device hypot. */
export function svgRadiusFromDeviceHypot(hypot: number): number {
  return (hypot / DEVICE_RADIUS_AT_RING_1) * MAP_RADIUS;
}

/** Zoom out so far misses stay visible with the whole face. */
export function fitFaceScale(shots: { x: number; y: number }[]): number {
  let maxR = VIEW_HALF;
  for (const s of shots) {
    const { cx, cy } = shotCoordsToSvg(s.x, s.y);
    maxR = Math.max(maxR, Math.hypot(cx - 50, cy - 50) + FIT_PAD);
  }
  return Math.min(1, VIEW_HALF / maxR);
}
