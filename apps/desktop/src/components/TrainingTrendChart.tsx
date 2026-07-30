import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { TrainingSessionSummary } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "./TargetFace";
import { fmtStat } from "../training/stats";

type Props = {
  sessions: TrainingSessionSummary[];
  metric: ScoreDisplayMode;
  average?: number | null;
  /** Open series detail when a point is activated. */
  onSelectSession?: (sessionId: string) => void;
};

function valueOf(s: TrainingSessionSummary, metric: ScoreDisplayMode): number {
  return metric === "teiler" ? s.teilerAvg : s.punkteTotal;
}

function finiteOr(v: number, fallback: number): number {
  return Number.isFinite(v) ? v : fallback;
}

function rollingAvg(values: number[], i: number, window: number): number {
  const start = Math.max(0, i - window + 1);
  const slice = values.slice(start, i + 1);
  return slice.reduce((a, b) => a + b, 0) / slice.length;
}

function shortDate(iso: string): string {
  const d = new Date(iso);
  if (!Number.isFinite(d.getTime())) return "—";
  return d.toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit" });
}

function fmtDelta(
  v: number,
  invert: boolean,
): { text: string; kind: "up" | "down" | "flat" } {
  if (Math.abs(v) < 0.05) return { text: "±0", kind: "flat" };
  const better = invert ? v < 0 : v > 0;
  const sign = v > 0 ? "+" : "";
  return {
    text: `${sign}${fmtStat(v)}`,
    kind: better ? "up" : "down",
  };
}

type FrameSize = { w: number; h: number };

/** Lightweight SVG trend chart — layout follows the measured frame size. */
export function TrainingTrendChart({
  sessions,
  metric,
  average,
  onSelectSession,
}: Props) {
  const [hover, setHover] = useState<number | null>(null);
  const [size, setSize] = useState<FrameSize>({ w: 640, h: 200 });
  const frameRef = useRef<HTMLDivElement>(null);
  const clipId = useId().replace(/:/g, "");
  const invertBetter = metric === "teiler";
  const label = metric === "teiler" ? "Ø Teiler" : "Σ Punkte";
  const rollWindow = Math.min(5, Math.max(3, Math.ceil(sessions.length / 3)));
  const density =
    sessions.length <= 4 ? "few" : sessions.length <= 14 ? "mid" : "many";

  useEffect(() => {
    const el = frameRef.current;
    if (!el) return;
    const apply = (w: number, h: number) => {
      const next = {
        w: Math.max(240, Math.round(w)),
        h: Math.max(110, Math.round(h)),
      };
      setSize((prev) =>
        prev.w === next.w && prev.h === next.h ? prev : next,
      );
    };
    apply(el.clientWidth, el.clientHeight);
    const ro = new ResizeObserver((entries) => {
      const cr = entries[0]?.contentRect;
      if (!cr || cr.width < 2 || cr.height < 2) return;
      apply(cr.width, cr.height);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [sessions.length]);

  const layout = useMemo(() => {
    const w = size.w;
    const h = size.h;
    const pad = {
      t: Math.max(14, Math.round(h * 0.07)),
      r: Math.max(12, Math.round(w * 0.02)),
      b: Math.max(28, Math.round(h * 0.14)),
      l: Math.max(40, Math.round(Math.min(56, w * 0.07))),
    };
    const innerW = Math.max(40, w - pad.l - pad.r);
    const innerH = Math.max(40, h - pad.t - pad.b);
    const values = sessions
      .map((s) => valueOf(s, metric))
      .map((v) => finiteOr(v, 0));
    const avg = finiteOr(
      average != null && Number.isFinite(average)
        ? average
        : values.length
          ? values.reduce((a, b) => a + b, 0) / values.length
          : 0,
      0,
    );
    const roll = values.map((_, i) => rollingAvg(values, i, rollWindow));

    const rawMin = values.length ? Math.min(...values, avg, ...roll) : 0;
    const rawMax = values.length ? Math.max(...values, avg, ...roll) : 1;
    const rawSpan = rawMax - rawMin || 1;
    const padY = rawSpan * 0.14;
    const min = rawMin - padY;
    const max = rawMax + padY;
    const span = max - min || 1;

    const xAt = (i: number) =>
      pad.l +
      (values.length <= 1 ? innerW / 2 : (i / (values.length - 1)) * innerW);
    const yAt = (v: number) => pad.t + innerH - ((v - min) / span) * innerH;

    const pts = values.map((v, i) => ({
      x: xAt(i),
      y: yAt(v),
      v,
      i,
      roll: roll[i]!,
    }));

    const line = pts
      .map((p, i) => `${i === 0 ? "M" : "L"}${p.x.toFixed(1)},${p.y.toFixed(1)}`)
      .join(" ");
    const rollLine = pts
      .map(
        (p, i) =>
          `${i === 0 ? "M" : "L"}${p.x.toFixed(1)},${yAt(p.roll).toFixed(1)}`,
      )
      .join(" ");
    const area =
      pts.length > 0
        ? `${line} L${pts[pts.length - 1]!.x.toFixed(1)},${(pad.t + innerH).toFixed(1)} L${pts[0]!.x.toFixed(1)},${(pad.t + innerH).toFixed(1)} Z`
        : "";

    const avgY = yAt(avg);
    const avgLabelBelow = avgY < pad.t + 14;
    const yTicks = [rawMin, (rawMin + rawMax) / 2, rawMax];

    let bestIdx = 0;
    let worstIdx = 0;
    for (let i = 1; i < values.length; i++) {
      const v = values[i]!;
      if (invertBetter ? v < values[bestIdx]! : v > values[bestIdx]!) bestIdx = i;
      if (invertBetter ? v > values[worstIdx]! : v < values[worstIdx]!)
        worstIdx = i;
    }

    // More x-labels when the frame is wide enough.
    const maxLabels = Math.max(
      2,
      Math.min(sessions.length, Math.floor(innerW / 72)),
    );
    const xLabelIdx: number[] = [];
    if (sessions.length <= maxLabels) {
      for (let i = 0; i < sessions.length; i++) xLabelIdx.push(i);
    } else if (sessions.length > 0) {
      for (let k = 0; k < maxLabels; k++) {
        xLabelIdx.push(
          Math.round((k / (maxLabels - 1)) * (sessions.length - 1)),
        );
      }
    }
    const xLabels = [...new Set(xLabelIdx)].sort((a, b) => a - b);

    let momentum: number | null = null;
    if (values.length >= 4) {
      const n = Math.max(2, Math.floor(values.length / 3));
      const recent = values.slice(-n);
      const earlier = values.slice(0, Math.min(n, values.length - n));
      if (earlier.length > 0) {
        momentum =
          recent.reduce((s, v) => s + v, 0) / recent.length -
          earlier.reduce((s, v) => s + v, 0) / earlier.length;
      }
    }

    const hitR = Math.max(
      12,
      Math.min(22, values.length > 1 ? innerW / (values.length - 1) / 2.2 : 20),
    );
    const fontAxis = Math.max(9, Math.min(12, h * 0.048));

    return {
      w,
      h,
      pad,
      innerW,
      innerH,
      pts,
      line,
      rollLine,
      area,
      avg,
      avgY,
      avgLabelBelow,
      yTicks,
      bestIdx,
      worstIdx,
      xLabels,
      yAt,
      momentum,
      high: values[bestIdx] ?? 0,
      low: values[worstIdx] ?? 0,
      last: values[values.length - 1] ?? 0,
      hitR,
      fontAxis,
      min,
      span,
    };
  }, [sessions, metric, average, rollWindow, invertBetter, size]);

  if (sessions.length === 0) {
    return (
      <div className="trend-empty">
        Noch keine Trainingsserien — in der Arena eine volle 10er-Serie schießen.
      </div>
    );
  }

  const tip = hover != null ? sessions[hover] : null;
  const tipVal = tip ? valueOf(tip, metric) : null;
  const tipDelta =
    tipVal != null ? fmtDelta(tipVal - layout.avg, invertBetter) : null;
  const mom =
    layout.momentum != null
      ? fmtDelta(layout.momentum, invertBetter)
      : null;

  return (
    <div className="trend-chart" data-density={density}>
      <div className="trend-stat-strip" aria-label="Kennzahlen Verlauf">
        <div className="trend-stat">
          <span className="trend-stat-label">Hoch</span>
          <strong>{fmtStat(layout.high)}</strong>
        </div>
        <div className="trend-stat">
          <span className="trend-stat-label">Tief</span>
          <strong>{fmtStat(layout.low)}</strong>
        </div>
        <div className="trend-stat">
          <span className="trend-stat-label">Ø</span>
          <strong>{fmtStat(layout.avg)}</strong>
        </div>
        <div className="trend-stat">
          <span className="trend-stat-label">Letzte</span>
          <strong>{fmtStat(layout.last)}</strong>
        </div>
        <div className={`trend-stat trend-stat-mom mom-${mom?.kind ?? "flat"}`}>
          <span className="trend-stat-label">Trend</span>
          <strong>{mom?.text ?? "—"}</strong>
        </div>
      </div>

      <div className="trend-chart-frame" ref={frameRef}>
        <svg
          viewBox={`0 0 ${layout.w} ${layout.h}`}
          width="100%"
          height="100%"
          preserveAspectRatio="none"
          overflow="hidden"
          role="img"
          aria-label={`Verlauf ${label}`}
        >
          <defs>
            <linearGradient id={`trendFill-${clipId}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="rgba(224, 118, 76, 0.42)" />
              <stop offset="55%" stopColor="rgba(224, 118, 76, 0.12)" />
              <stop offset="100%" stopColor="rgba(224, 118, 76, 0)" />
            </linearGradient>
            <linearGradient
              id={`trendStroke-${clipId}`}
              x1="0"
              y1="0"
              x2="1"
              y2="0"
            >
              <stop offset="0%" stopColor="#e0764c" stopOpacity="0.75" />
              <stop offset="100%" stopColor="#e8987a" />
            </linearGradient>
            <clipPath id={`trendClip-${clipId}`}>
              <rect
                x={layout.pad.l}
                y={layout.pad.t}
                width={layout.innerW}
                height={layout.innerH}
                rx="8"
              />
            </clipPath>
          </defs>

          <rect
            x={layout.pad.l}
            y={layout.pad.t}
            width={layout.innerW}
            height={layout.innerH}
            rx="8"
            className="trend-plot"
          />

          {layout.yTicks.map((t, i) => {
            const y = layout.yAt(t);
            return (
              <g key={i}>
                <line
                  x1={layout.pad.l}
                  x2={layout.pad.l + layout.innerW}
                  y1={y}
                  y2={y}
                  className="trend-grid"
                />
                <text
                  x={layout.pad.l - 8}
                  y={y + 3.5}
                  textAnchor="end"
                  className="trend-axis"
                  fontSize={layout.fontAxis}
                >
                  {fmtStat(t)}
                </text>
              </g>
            );
          })}

          <g clipPath={`url(#trendClip-${clipId})`}>
            <line
              x1={layout.pad.l}
              x2={layout.pad.l + layout.innerW}
              y1={layout.avgY}
              y2={layout.avgY}
              className="trend-avg"
            />
            {layout.area ? (
              <path d={layout.area} fill={`url(#trendFill-${clipId})`} />
            ) : null}
            <path d={layout.rollLine} className="trend-roll" fill="none" />
            <path
              d={layout.line}
              className="trend-line"
              fill="none"
              stroke={`url(#trendStroke-${clipId})`}
            />

            {hover != null && layout.pts[hover] ? (
              <line
                x1={layout.pts[hover].x}
                x2={layout.pts[hover].x}
                y1={layout.pad.t}
                y2={layout.pad.t + layout.innerH}
                className="trend-crosshair"
              />
            ) : null}

            {layout.pts.map((p) => {
              const isBest = p.i === layout.bestIdx;
              const isWorst =
                p.i === layout.worstIdx && layout.bestIdx !== layout.worstIdx;
              const isActive = hover === p.i;
              const isLast = p.i === layout.pts.length - 1;
              return (
                <g key={p.i}>
                  <circle
                    cx={p.x}
                    cy={p.y}
                    r={layout.hitR}
                    className="trend-hit"
                    onMouseEnter={() => setHover(p.i)}
                    onMouseLeave={() => setHover(null)}
                    onClick={() => onSelectSession?.(sessions[p.i]!.id)}
                    style={{
                      cursor: onSelectSession ? "pointer" : "default",
                    }}
                  />
                  <circle
                    cx={p.x}
                    cy={p.y}
                    r={isActive ? 6 : isBest || isLast ? 5 : 3.6}
                    className={[
                      "trend-dot",
                      isActive ? "active" : "",
                      isBest ? "is-best" : "",
                      isWorst ? "is-worst" : "",
                      isLast ? "is-last" : "",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    style={{ pointerEvents: "none" }}
                  />
                </g>
              );
            })}
          </g>

          <text
            x={layout.pad.l + 8}
            y={layout.avgLabelBelow ? layout.avgY + 13 : layout.avgY - 6}
            className="trend-avg-label"
            fontSize={layout.fontAxis}
          >
            Ø {fmtStat(layout.avg)}
          </text>
          <text
            x={layout.pad.l + layout.innerW - 4}
            y={layout.pad.t + Math.max(12, layout.fontAxis + 2)}
            textAnchor="end"
            className="trend-roll-label"
            fontSize={layout.fontAxis}
          >
            Ø{rollWindow} gleitend
          </text>

          {layout.xLabels.map((i) => {
            const s = sessions[i]!;
            const x = layout.pts[i]!.x;
            return (
              <text
                key={`x-${i}`}
                x={x}
                y={layout.h - Math.max(8, layout.pad.b * 0.28)}
                textAnchor="middle"
                className="trend-axis trend-x-label"
                fontSize={layout.fontAxis}
              >
                {shortDate(s.endedAt)}
              </text>
            );
          })}
        </svg>
      </div>

      <div className="trend-legend" aria-hidden>
        <span className="trend-legend-item">
          <i className="trend-legend-swatch trend-legend-series" /> Serie
        </span>
        <span className="trend-legend-item">
          <i className="trend-legend-swatch trend-legend-roll" /> Gleitender Ø
        </span>
        <span className="trend-legend-item">
          <i className="trend-legend-swatch trend-legend-avg" /> Gesamt-Ø
        </span>
        <span className="trend-legend-item">
          <i className="trend-legend-swatch trend-legend-best" /> Bestwert
        </span>
      </div>

      {tip && tipVal != null && tipDelta ? (
        <p className="trend-tip">
          <strong>{fmtStat(tipVal)}</strong>
          <span className={`trend-tip-delta tip-${tipDelta.kind}`}>
            {tipDelta.text} vs Ø
          </span>
          <span className="trend-tip-sep">·</span>
          {tip.shooterName}
          <span className="trend-tip-sep">·</span>
          {tip.shotCount} Schüsse
          <span className="trend-tip-sep">·</span>
          {new Date(tip.endedAt).toLocaleString("de-DE")}
          {onSelectSession ? (
            <span className="trend-tip-action"> · Klick öffnet Serie</span>
          ) : null}
        </p>
      ) : (
        <p className="trend-tip muted">
          Punkt anfahren für Details
          {onSelectSession ? " · Klick öffnet die Serie" : ""}
        </p>
      )}
    </div>
  );
}
