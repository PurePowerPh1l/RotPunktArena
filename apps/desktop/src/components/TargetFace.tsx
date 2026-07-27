import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { UiShot } from "@rotpunktarena/domain";
import {
  BLACK_AIM_RADIUS_SVG,
  DEVICE_RADIUS_AT_RING_1,
  FACE_BLACK_FILL,
  FACE_PAPER_FILL,
  fitFaceScale,
  MAP_RADIUS,
  RING_LABEL_FONT_SIZE_SVG,
  scoringRingLabelSpecs,
  scoringRingSpecs,
  shotCoordsToSvg,
  VIEW_HALF,
} from "./targetFaceGeometry";

export {
  DEVICE_RADIUS_AT_RING_1,
  fitFaceScale,
  MAP_RADIUS,
  scoringRingOuterSvg,
  scoringRingSpecs,
  shotCoordsToSvg,
} from "./targetFaceGeometry";

const SCORING_RINGS = scoringRingSpecs();
const RING_LABELS = scoringRingLabelSpecs();

/**
 * Mouse/sim only — keeps click-to-shoot near the paper.
 * Same radius as Rust `AIM_RADIUS_AT_RING_1` / device face mapping.
 * Real Reddot light-gun shots are not clamped and may zoom the face far out.
 */
const MOUSE_AIM_MAX_R = DEVICE_RADIUS_AT_RING_1 * 1.12;

/** Absolute view scale limits (paper size on the mat), not just the user multiplier. */
const INSPECT_VIEW_MIN = 0.42;
const INSPECT_VIEW_MAX = 12;
const INSPECT_ZOOM_STEP = 1.18;

export type FaceLabelMode = "off" | "index" | "value";
export type ScoreDisplayMode = "punkte" | "teiler";

type Props = {
  shots: UiShot[];
  /** Most recent shot — always highlighted while shooting. */
  last?: UiShot | null;
  /** Best shot for current metric — only when series is complete. */
  best?: UiShot | null;
  /** User-selected shot in end-screen inspect mode. */
  focusShot?: number | null;
  onFocusShot?: (shotIndex: number | null) => void;
  onAimClick?: (x: number, y: number) => void;
  interactive?: boolean;
  /** What to annotate next to impact marks. */
  labelMode?: FaceLabelMode;
  /** Which metric to show when labelMode is "value". */
  displayMode?: ScoreDisplayMode;
  /**
   * After a series: mouse-wheel zoom + drag pan to inspect impacts.
   * Resets automatically when this becomes false (new series).
   */
  allowInspect?: boolean;
};

type InspectView = { zoom: number; panX: number; panY: number };

const INSPECT_RESET: InspectView = { zoom: 1, panX: 0, panY: 0 };

/** How far the view may pan at a given absolute view scale (viewBox units). */
function maxPanForViewScale(viewScale: number): number {
  // Always allow some drag at 100%; more room when zoomed in.
  const paperR = 48 * viewScale;
  const room = Math.max(18, paperR - 8);
  return Math.min(58, room + (viewScale > 1 ? 10 * (viewScale - 1) : 0));
}

/** Mat behind the paper — must match `.face-wrap` so zoom never reveals a light fringe. */
const FACE_MAT = "#0c0e12";

/** Rounded integers on the face only — table keeps decimals. */
function formatFaceValue(v: number): string {
  return String(Math.round(v));
}

/** Slightly smaller dots when dense, but never so small they vanish. */
function markRadius(emphasized: boolean, shotCount: number): number {
  if (emphasized) return 1.85;
  if (shotCount > 48) return 1.15;
  if (shotCount > 24) return 1.25;
  return 1.35;
}

function clampAimRadius(x: number, y: number, maxR: number): { x: number; y: number } {
  const r = Math.hypot(x, y);
  if (r <= maxR || r === 0) return { x, y };
  const t = maxR / r;
  return { x: x * t, y: y * t };
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n));
}

export function TargetFace({
  shots,
  last,
  best,
  focusShot = null,
  onFocusShot,
  onAimClick,
  interactive,
  labelMode = "value",
  displayMode = "punkte",
  allowInspect = false,
}: Props) {
  const glowId = useId();
  const svgRef = useRef<SVGSVGElement>(null);
  const dense = shots.length > 24;
  const fitScale = useMemo(() => fitFaceScale(shots), [shots]);
  const [inspect, setInspect] = useState<InspectView>(INSPECT_RESET);
  const [resetting, setResetting] = useState(false);
  const dragRef = useRef<{
    x: number;
    y: number;
    panX: number;
    panY: number;
    moved: boolean;
    shotId: number | null;
  } | null>(null);

  // New series / leave inspect → snap back to fit view.
  useEffect(() => {
    if (!allowInspect) setInspect(INSPECT_RESET);
  }, [allowInspect]);

  const viewScale = fitScale * inspect.zoom;
  const marks = useMemo(
    () =>
      shots.map((s) => {
        const active = last?.shotIndex === s.shotIndex;
        const isBest = best != null && best.shotIndex === s.shotIndex;
        const focused = focusShot != null && focusShot === s.shotIndex;
        const showLabel =
          labelMode !== "off" &&
          (active ||
            isBest ||
            focused ||
            s.shotIndex > shots.length - (dense ? 6 : 10) ||
            (!dense && shots.length <= 14));
        const text =
          labelMode === "index"
            ? String(s.shotIndex)
            : labelMode === "value"
              ? formatFaceValue(displayMode === "teiler" ? s.distanceDisplay : s.valueDisplay)
              : "";
        const { cx, cy } = shotCoordsToSvg(s.x, s.y);
        return {
          id: s.shotIndex,
          cx,
          cy,
          active,
          isBest,
          focused,
          showLabel,
          text,
        };
      }),
    [shots, last, best, focusShot, dense, labelMode, displayMode],
  );

  // Non-passive wheel so we can prevent page scroll while inspecting.
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg || !allowInspect) return;

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();
      // Avoid accidental text selection of label numbers while zooming.
      window.getSelection()?.removeAllRanges();
      const rect = svg.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) return;
      const mx = ((e.clientX - rect.left) / rect.width) * 100;
      const my = ((e.clientY - rect.top) / rect.height) * 100;

      setInspect((prev) => {
        const factor = e.deltaY < 0 ? INSPECT_ZOOM_STEP : 1 / INSPECT_ZOOM_STEP;
        const s0 = fitScale * prev.zoom;
        // Clamp by absolute paper size so far-miss fit + zoom-out don't collapse to a speck.
        const s1 = clamp(s0 * factor, INSPECT_VIEW_MIN, INSPECT_VIEW_MAX);
        const nextZoom = s1 / fitScale;
        if (Math.abs(nextZoom - prev.zoom) < 0.0001) return prev;
        // Keep the point under the cursor stable across zoom.
        const contentX = (mx - 50 - prev.panX) / s0;
        const contentY = (my - 50 - prev.panY) / s0;
        const panX = mx - 50 - contentX * s1;
        const panY = my - 50 - contentY * s1;
        const maxPan = maxPanForViewScale(s1);
        return {
          zoom: nextZoom,
          panX: clamp(panX, -maxPan, maxPan),
          panY: clamp(panY, -maxPan, maxPan),
        };
      });
    };

    const onSelectStart = (e: Event) => e.preventDefault();

    svg.addEventListener("wheel", onWheel, { passive: false });
    svg.addEventListener("selectstart", onSelectStart);
    return () => {
      svg.removeEventListener("wheel", onWheel);
      svg.removeEventListener("selectstart", onSelectStart);
    };
  }, [allowInspect, fitScale]);

  const handleClick = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!interactive || !onAimClick) return;
    if (Math.abs(inspect.zoom - 1) > 0.02) return; // inspecting — don't fire
    e.preventDefault();
    const svg = e.currentTarget;
    const rect = svg.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return;
    const localX = ((e.clientX - rect.left) / rect.width) * 100;
    const localY = ((e.clientY - rect.top) / rect.height) * 100;
    const contentX = 50 + (localX - 50) / fitScale;
    const contentY = 50 + (localY - 50) / fitScale;
    // Inverse of shotCoordsToSvg — same DEVICE_RADIUS_AT_RING_1 as Rust aim_coords_to_ascii.
    const rawX = ((contentX - 50) / MAP_RADIUS) * DEVICE_RADIUS_AT_RING_1;
    const rawY = ((50 - contentY) / MAP_RADIUS) * DEVICE_RADIUS_AT_RING_1;
    const { x, y } = clampAimRadius(rawX, rawY, MOUSE_AIM_MAX_R);
    onAimClick(x, y);
  };

  const hitShotAt = (clientX: number, clientY: number): number | null => {
    const svg = svgRef.current;
    if (!svg) return null;
    const rect = svg.getBoundingClientRect();
    if (rect.width <= 0) return null;
    const mx = ((clientX - rect.left) / rect.width) * 100;
    const my = ((clientY - rect.top) / rect.height) * 100;
    // Inverse of face-scene transform.
    const contentX = (mx - 50 - inspect.panX) / viewScale + 50;
    const contentY = (my - 50 - inspect.panY) / viewScale + 50;
    const hitR = 3.2 / Math.max(viewScale, 0.35);
    let bestId: number | null = null;
    let bestDist = hitR;
    for (const m of marks) {
      const d = Math.hypot(m.cx - contentX, m.cy - contentY);
      if (d < bestDist) {
        bestDist = d;
        bestId = m.id;
      }
    }
    return bestId;
  };

  const onPointerDown = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!allowInspect) return;
    if (e.button !== 0) return;
    // Don't leave a focus ring on the square SVG (shows as light rims on the circle).
    e.preventDefault();
    const shotId = hitShotAt(e.clientX, e.clientY);
    e.currentTarget.setPointerCapture(e.pointerId);
    dragRef.current = {
      x: e.clientX,
      y: e.clientY,
      panX: inspect.panX,
      panY: inspect.panY,
      moved: false,
      shotId,
    };
  };

  const onPointerMove = (e: React.PointerEvent<SVGSVGElement>) => {
    const drag = dragRef.current;
    if (!drag || !allowInspect) return;
    const svg = e.currentTarget;
    const rect = svg.getBoundingClientRect();
    if (rect.width <= 0) return;
    const dxPx = e.clientX - drag.x;
    const dyPx = e.clientY - drag.y;
    if (!drag.moved && Math.hypot(dxPx, dyPx) > 6) drag.moved = true;
    if (!drag.moved) return;
    const dx = (dxPx / rect.width) * 100;
    const dy = (dyPx / rect.height) * 100;
    const maxPan = maxPanForViewScale(viewScale);
    setInspect((prev) => ({
      ...prev,
      panX: clamp(drag.panX + dx, -maxPan, maxPan),
      panY: clamp(drag.panY + dy, -maxPan, maxPan),
    }));
  };

  const onPointerUp = (e: React.PointerEvent<SVGSVGElement>) => {
    const drag = dragRef.current;
    if (drag) {
      if (!drag.moved && onFocusShot) {
        if (drag.shotId != null) {
          onFocusShot(focusShot === drag.shotId ? null : drag.shotId);
        } else {
          onFocusShot(null);
        }
      }
      dragRef.current = null;
      try {
        e.currentTarget.releasePointerCapture(e.pointerId);
      } catch {
        /* ignore */
      }
    }
  };

  const onDoubleClick = () => {
    if (!allowInspect) return;
    setResetting(true);
    setInspect(INSPECT_RESET);
    window.setTimeout(() => setResetting(false), 380);
  };

  const inspecting =
    allowInspect &&
    (Math.abs(inspect.zoom - 1) > 0.02 || Math.hypot(inspect.panX, inspect.panY) > 0.5);
  const aria = interactive
    ? "Scheibe — Klick zum Schießen"
    : allowInspect
      ? "Scheibe — Schuss antippen, Mausrad zoomen, ziehen verschieben"
      : "Scheibe";

  return (
    <svg
      ref={svgRef}
      className={`face${interactive ? " face-interactive" : ""}${dense ? " face-dense" : ""}${fitScale < 0.999 ? " face-zoomed" : ""}${allowInspect ? " face-inspectable" : ""}${inspecting ? " face-inspecting" : ""}${resetting ? " face-inspect-reset" : ""}`}
      viewBox="0 0 100 100"
      overflow="hidden"
      aria-label={aria}
      role={interactive ? "button" : "img"}
      tabIndex={interactive ? 0 : undefined}
      onClick={handleClick}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onDoubleClick={onDoubleClick}
      onKeyDown={(e) => {
        if (!interactive || !onAimClick) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onAimClick(0, 0);
        }
      }}
    >
      <defs>
        <radialGradient id={glowId} cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#f3e8d6" />
          <stop offset="55%" stopColor={FACE_PAPER_FILL} />
          <stop offset="100%" stopColor="#c9b896" />
        </radialGradient>
        <clipPath id={`${glowId}-port`}>
          <circle cx="50" cy="50" r="50" />
        </clipPath>
      </defs>
      {/* Fixed mat — does not scale with inspect zoom (avoids light clip fringes). */}
      <circle cx="50" cy="50" r="50.75" fill={FACE_MAT} />
      <g clipPath={`url(#${glowId}-port)`}>
      <g
        className="face-scene"
        style={{
          transform: `translate(${inspect.panX}%, ${inspect.panY}%) scale(${viewScale})`,
        }}
      >
        <circle
          cx="50"
          cy="50"
          r={VIEW_HALF}
          fill={`url(#${glowId})`}
          stroke="#2a2a28"
          strokeWidth="0.6"
        />
        <circle cx="50" cy="50" r={BLACK_AIM_RADIUS_SVG} fill={FACE_BLACK_FILL} />
        {SCORING_RINGS.map((spec) => (
          <circle
            key={spec.ring}
            cx="50"
            cy="50"
            r={spec.outerSvg}
            fill="none"
            stroke={spec.onBlack ? "rgba(245,240,230,0.55)" : "#3a3a36"}
            strokeWidth="0.35"
          />
        ))}
        {RING_LABELS.flatMap((spec) =>
          spec.positions.map((p) => (
            <text
              key={`ring-${spec.ring}-${p.axis}`}
              x={p.cx}
              y={p.cy}
              textAnchor="middle"
              dominantBaseline="central"
              fontSize={RING_LABEL_FONT_SIZE_SVG}
              fontWeight={700}
              fontFamily="Barlow Condensed, Arial Narrow, sans-serif"
              fill={spec.onBlack ? "rgba(245,240,230,0.92)" : "#2a241c"}
              stroke={
                spec.onBlack ? "rgba(0,0,0,0.55)" : "rgba(232,220,200,0.85)"
              }
              strokeWidth={0.35}
              paintOrder="stroke fill"
              style={{ pointerEvents: "none", userSelect: "none" }}
            >
              {spec.ring}
            </text>
          )),
        )}
        {marks.map((m) => {
          const emphasized = m.active || m.isBest || m.focused;
          const r = markRadius(emphasized, shots.length) + (m.focused ? 0.55 : 0);
          const fill = m.focused
            ? "#1f6feb"
            : m.active
              ? "#c0392b"
              : m.isBest
                ? "#b8860b"
                : "#4a3228";
          const stroke = m.focused ? "#fff" : emphasized ? "#fff" : "#f5f0e6";
          const dimmed = focusShot != null && !m.focused;
          const cls = m.focused
            ? "shot-mark-focus"
            : m.active
              ? "shot-mark-active"
              : m.isBest
                ? "shot-mark-best"
                : "shot-mark";
          return (
            <circle
              key={`dot-${m.id}`}
              cx={m.cx}
              cy={m.cy}
              r={r}
              fill={fill}
              stroke={stroke}
              strokeWidth={m.focused ? 0.55 : emphasized ? 0.4 : 0.5}
              opacity={dimmed ? 0.38 : 1}
              className={cls}
              style={{ pointerEvents: "none" }}
            />
          );
        })}
        {marks.map((m) => {
          if (!m.showLabel) return null;
          const fontSize = labelMode === "value" ? 2.8 : 2.5;
          // Barlow Condensed digits are narrow; keep pills tight to the glyphs.
          const charW = fontSize * 0.42;
          const padX = 0.55;
          const padY = 0.35;
          const digits = Math.max(m.text.length, 1);
          const boxW = digits * charW + padX * 2;
          const boxH = fontSize + padY * 2;
          const lx = m.cx + 2.2 + boxW / 2;
          const ly = m.cy - 1.2;
          const pillClass = m.focused
            ? "face-label-pill face-label-pill-focus"
            : m.active
              ? "face-label-pill face-label-pill-active"
              : m.isBest
                ? "face-label-pill face-label-pill-best"
                : "face-label-pill";
          const stroke = m.focused
            ? "#1f6feb"
            : m.active
              ? "#c0392b"
              : m.isBest
                ? "#b8860b"
                : "rgba(42,42,40,0.35)";
          const dimmed = focusShot != null && !m.focused;
          return (
            <g
              key={`lbl-${m.id}`}
              style={{ pointerEvents: "none" }}
              opacity={dimmed ? 0.4 : 1}
            >
              <rect
                className={pillClass}
                x={lx - boxW / 2}
                y={ly - boxH / 2}
                width={boxW}
                height={boxH}
                rx={1.0}
                ry={1.0}
                fill="rgba(245,240,230,0.92)"
                stroke={stroke}
                strokeWidth={0.35}
              />
              <text
                x={lx}
                y={ly}
                textAnchor="middle"
                dominantBaseline="central"
                fontSize={fontSize}
                fontWeight="600"
                fill="#1a1a18"
                fontFamily="Barlow Condensed, sans-serif"
              >
                {m.text}
              </text>
            </g>
          );
        })}
      </g>
      </g>
    </svg>
  );
}
