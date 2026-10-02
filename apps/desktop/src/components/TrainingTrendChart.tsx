import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { TrainingSessionSummary } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "./TargetFace";
import { fmtStat } from "../training/stats";
import { tenShotPoints } from "../training/comparison";

type Props = {
  sessions: TrainingSessionSummary[];
  metric: ScoreDisplayMode;
  average?: number | null;
  emptyMessage?: string;
  onSelectSession?: (sessionId: string) => void;
};

export function TrainingTrendChart({ sessions, metric, average, emptyMessage = "Noch keine Trainingsserien — in der Arena eine vollständige Serie schießen.", onSelectSession }: Props) {
  const [selected, setSelected] = useState<number | null>(null);
  const [hover, setHover] = useState<number | null>(null);
  const [comparisons, setComparisons] = useState(false);
  const [size, setSize] = useState({ w: 640, h: 220 });
  const frameRef = useRef<HTMLDivElement>(null);
  const clipId = useId().replace(/:/g, "");
  const teiler = metric === "teiler";
  const unit = teiler ? "Ø Teiler" : "Punkte / 10 Schüsse";

  useEffect(() => { setSelected(null); setHover(null); }, [sessions]);
  useEffect(() => {
    const element = frameRef.current;
    if (!element) return;
    const update = () => setSize({ w: Math.max(240, element.clientWidth), h: Math.max(160, element.clientHeight) });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [sessions.length]);

  const layout = useMemo(() => {
    const values = sessions.map(s => teiler ? s.teilerAvg : tenShotPoints(s)).map(v => Number.isFinite(v) ? v : 0);
    const avg = average != null && Number.isFinite(average) ? average : values.reduce((sum, v) => sum + v, 0) / (values.length || 1);
    const best = values.length ? (teiler ? Math.min(...values) : Math.max(...values)) : 0;
    const low = Math.min(...values, avg), high = Math.max(...values, avg);
    const padding = Math.max((high - low) * 0.16, teiler ? 1 : 0.5);
    const min = Math.max(0, low - padding), max = high + padding;
    const axisWidth = Math.max(56, Math.max(fmtStat(low).length, fmtStat(high).length) * 7 + 14);
    const plot = { x: axisWidth, y: 16, w: Math.max(40, size.w - axisWidth - 24), h: size.h - 52 };
    const y = (v: number) => plot.y + plot.h - (v - min) / (max - min || 1) * plot.h;
    const points = values.map((value, i) => ({ x: plot.x + 8 + (values.length <= 1 ? (plot.w - 16) / 2 : i / (values.length - 1) * (plot.w - 16)), y: y(value), value, i }));
    const path = (pts: {x: number; y: number}[]) => pts.map((p,i) => `${i ? "L" : "M"}${p.x},${p.y}`).join(" ");
    const rolling = points.slice(4).map(p => ({x: p.x, y: y(values.slice(p.i - 4,p.i + 1).reduce((sum,v) => sum + v,0) / 5)}));
    const labelCount = Math.max(2, Math.min(6, Math.floor(plot.w / 80)));
    const labels = points.filter(p => values.length <= labelCount || p.i === 0 || p.i === values.length - 1 || p.i % Math.ceil((values.length - 1) / (labelCount - 1)) === 0);
    const ticks = high === low ? [low] : [low, (low + high) / 2, high];
    return { values, avg, best, plot, points, line: path(points), rolling: path(rolling), labels, ticks, y };
  }, [sessions, teiler, average, size]);

  if (!sessions.length) return <div className="trend-empty">{emptyMessage}</div>;
  const active = Math.min(hover ?? selected ?? sessions.length - 1, sessions.length - 1);
  const series = sessions[active]!;
  const value = layout.values[active]!;
  const firstDate = new Date(sessions[0]!.endedAt).toLocaleDateString("de-DE");
  const lastDate = new Date(sessions[sessions.length - 1]!.endedAt).toLocaleDateString("de-DE");
  const delta = value - layout.avg;
  const better = teiler ? delta < 0 : delta > 0;
  const choose = (index: number) => { setHover(null); setSelected(Math.max(0, Math.min(sessions.length - 1, index))); };

  return <div className="trend-chart trend-chart-clear">
    <div className="trend-overview">
      <div className="trend-stat-strip" aria-label="Kennzahlen Verlauf">
        <div className="trend-stat"><span className="trend-stat-label">Letzte Serie</span><strong>{fmtStat(layout.values[layout.values.length - 1]!)}</strong></div>
        <div className="trend-stat"><span className="trend-stat-label">Durchschnitt</span><strong>{fmtStat(layout.avg)}</strong></div>
        <div className="trend-stat"><span className="trend-stat-label">Beste Serie</span><strong>{fmtStat(layout.best)}</strong></div>
      </div>
      <p className="trend-explanation"><strong>{unit}</strong><span>{teiler ? "Kleiner ist besser" : "Höher ist besser"} · {sessions.length} {sessions.length === 1 ? "Serie" : "Serien"}</span><span>{firstDate === lastDate ? firstDate : `${firstDate} – ${lastDate}`}</span></p>
    </div>
    <div className="trend-chart-toolbar">
      <span>Jeder Punkt ist eine Serie · von links nach rechts</span>
      {sessions.length > 1 ? <label className="check-field"><input type="checkbox" checked={comparisons} onChange={e => setComparisons(e.target.checked)} />Vergleichslinien</label> : null}
    </div>
    <div className="trend-chart-frame" ref={frameRef}>
      <svg viewBox={`0 0 ${size.w} ${size.h}`} role="group" aria-label={`Leistungsverlauf: ${unit}, älteste bis neueste Serie`}>
        <defs><clipPath id={clipId}><rect x={layout.plot.x} y={layout.plot.y} width={layout.plot.w} height={layout.plot.h} /></clipPath></defs>
        <rect {...{x: layout.plot.x,y:layout.plot.y,width:layout.plot.w,height:layout.plot.h}} rx="8" className="trend-plot" />
        {layout.ticks.map((tick,i) => <g key={i}><line x1={layout.plot.x} x2={layout.plot.x + layout.plot.w} y1={layout.y(tick)} y2={layout.y(tick)} className="trend-grid" /><text x={layout.plot.x - 9} y={layout.y(tick) + 4} textAnchor="end" className="trend-axis">{fmtStat(tick)}</text></g>)}
        <g clipPath={`url(#${clipId})`}>
          {comparisons && sessions.length > 1 ? <><line x1={layout.plot.x} x2={layout.plot.x + layout.plot.w} y1={layout.y(layout.avg)} y2={layout.y(layout.avg)} className="trend-avg" />{sessions.length >= 5 ? <path d={layout.rolling} className="trend-roll" /> : null}</> : null}
          <path d={layout.line} className="trend-line" stroke="var(--chart-series)" />
          <line x1={layout.points[active]!.x} x2={layout.points[active]!.x} y1={layout.plot.y} y2={layout.plot.y + layout.plot.h} className="trend-crosshair" />
          {layout.points.map(p => <g key={sessions[p.i]!.id}>
            {(active === p.i || layout.plot.w / Math.max(1, sessions.length - 1) >= 10) ? <circle cx={p.x} cy={p.y} r={active === p.i ? 5.5 : 3.5} className={`trend-dot${active === p.i ? " active" : ""}`} style={{pointerEvents:"none"}} /> : null}
            <circle cx={p.x} cy={p.y} r={Math.max(5, Math.min(16,layout.plot.w / sessions.length / 2))} className="trend-hit" role="button" tabIndex={active === p.i ? 0 : -1} aria-label={`Serie ${p.i + 1}, ${fmtStat(p.value)} ${unit}, ${new Date(sessions[p.i]!.endedAt).toLocaleString("de-DE")}`} aria-pressed={active === p.i} onMouseEnter={() => setHover(p.i)} onMouseLeave={() => setHover(null)} onClick={() => choose(p.i)} onFocus={() => choose(p.i)} onKeyDown={e => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); choose(active + (e.key === "ArrowRight" ? 1 : -1)); } else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); choose(p.i); } }} />
          </g>)}
        </g>
        {layout.labels.map(p => <text key={p.i} x={p.x} y={size.h - 12} textAnchor="middle" className="trend-axis trend-x-label">{p.i + 1}</text>)}
      </svg>
    </div>
    <div className="trend-chart-bottom"><span>Serienfolge · älteste → neueste</span>{comparisons && sessions.length > 1 ? <div className="trend-legend"><span className="trend-legend-item"><i className="trend-legend-swatch trend-legend-avg" />Durchschnitt</span>{sessions.length >= 5 ? <span className="trend-legend-item"><i className="trend-legend-swatch trend-legend-roll" />Schnitt der letzten 5 Serien</span> : null}</div> : null}</div>
    <div className="trend-selection">
      <div className="trend-selection-value"><span>Serie {active + 1} von {sessions.length}</span><strong>{fmtStat(value)} <small>{unit}</small></strong><span className={Math.abs(delta) < .05 ? "" : better ? "trend-selection-up" : "trend-selection-down"}>{delta > 0 ? "+" : ""}{fmtStat(delta)} zum Durchschnitt</span></div>
      <div className="trend-selection-meta"><strong>{series.shooterName}</strong><span>{new Date(series.endedAt).toLocaleString("de-DE",{dateStyle:"medium",timeStyle:"short"})}</span><span>{series.shotCount} Schüsse{!teiler && series.shotCount !== 10 ? ` · ${fmtStat(series.punkteTotal)} Punkte, auf 10 Schüsse umgerechnet` : ""}</span></div>
      <div className="trend-selection-actions"><button type="button" className="hist-small-action hist-icon-action" disabled={active === 0} onClick={() => choose(active - 1)} aria-label="Vorherige Serie">‹</button><button type="button" className="hist-small-action hist-icon-action" disabled={active === sessions.length - 1} onClick={() => choose(active + 1)} aria-label="Nächste Serie">›</button>{onSelectSession ? <button type="button" className="hist-small-action" onClick={() => onSelectSession(series.id)}>Serie öffnen</button> : null}</div>
    </div>
  </div>;
}
