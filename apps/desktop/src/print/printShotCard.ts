import type { UiShot } from "@reddot/domain";
import { type ScoreDisplayMode } from "../components/TargetFace";
import {
  fitFaceScale,
  scoringFaceChromeSvg,
  shotCoordsToSvg,
} from "../components/targetFaceGeometry";
import { bestShotOf } from "../hooks/useScoreDisplay";
import { escapeHtml, openPrintHtml, PRINT_BASE_CSS } from "./printHtml";

export type ShotCardPrintInput = {
  shooterName: string;
  modeLabel: string;
  shots: UiShot[];
  seriesTotal: number;
  maxShots?: number | null;
  /** Which metric defines the best shot. Default punkte. */
  displayMode?: ScoreDisplayMode;
  printedAt?: Date;
};

function fmt(v: number): string {
  return Number.isInteger(v) ? String(v) : v.toFixed(1);
}

function markSvg(shots: UiShot[], displayMode: ScoreDisplayMode): string {
  const scale = fitFaceScale(shots);
  const lastIdx = shots[shots.length - 1]?.shotIndex;
  const best = bestShotOf(shots, displayMode);
  const marks = shots
    .map((s) => {
      const { cx, cy } = shotCoordsToSvg(s.x, s.y);
      const active = s.shotIndex === lastIdx;
      const isBest = best != null && s.shotIndex === best.shotIndex;
      const hi = active || isBest;
      const fill = active ? "#b33" : isBest ? "#b8860b" : "#111";
      return (
        `<g>` +
        `<circle cx="${cx}" cy="${cy}" r="${hi ? 1.7 : 1.2}" fill="${fill}" stroke="${hi ? "#fff" : "none"}" stroke-width="0.35"/>` +
        `<text x="${cx + 2.2}" y="${cy - 1.5}" font-size="2.4" fill="#222" font-family="Segoe UI,sans-serif">${s.shotIndex}</text>` +
        `</g>`
      );
    })
    .join("");
  return `
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" width="320" height="320">
  <g transform="translate(50 50) scale(${scale}) translate(-50 -50)">
  ${scoringFaceChromeSvg()}
  ${marks}
  </g>
</svg>`;
}

function buildHtml(input: ShotCardPrintInput): string {
  const when = (input.printedAt ?? new Date()).toLocaleString("de-DE");
  const displayMode = input.displayMode ?? "punkte";
  const best = bestShotOf(input.shots, displayMode);
  const rows = input.shots
    .map((s) => {
      const isBest = best != null && s.shotIndex === best.shotIndex;
      const note = isBest ? " <strong>(Best)</strong>" : "";
      return `<tr${isBest ? ' class="best"' : ""}><td>${s.shotIndex}${note}</td><td>${fmt(s.valueDisplay)}</td><td>${fmt(s.distanceDisplay)}</td><td>${fmt(s.seriesTotal)}</td><td>${s.x} / ${s.y}</td></tr>`;
    })
    .join("");
  const limit =
    input.maxShots != null ? ` · Limit ${input.shots.length}/${input.maxShots}` : "";

  return `<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8"/>
<title>Schussbild — ${escapeHtml(input.shooterName)}</title>
<style>
${PRINT_BASE_CSS}
  .grid {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: 1.25rem;
    align-items: start;
  }
  .total { margin-top: 0.85rem; font-size: 1.1rem; }
  .total strong { font-size: 1.35rem; }
  tr.best td { font-weight: 700; }
</style>
</head>
<body>
  <button class="noprint" onclick="window.print()" style="margin-bottom:1rem;padding:0.5rem 0.9rem;">Drucken</button>
  <h1>Schussbild — ${escapeHtml(input.shooterName)}</h1>
  <p class="meta">${escapeHtml(input.modeLabel)}${limit} · ${escapeHtml(when)}</p>
  <div class="grid">
    <div>${markSvg(input.shots, displayMode)}</div>
    <div>
      <table>
        <thead><tr><th>#</th><th>Wert</th><th>Teiler</th><th>Σ</th><th>X / Y</th></tr></thead>
        <tbody>${rows || "<tr><td colspan='5'>Keine Schüsse</td></tr>"}</tbody>
      </table>
      <p class="total">Serie <strong>${fmt(input.seriesTotal)}</strong></p>
    </div>
  </div>
  <p class="foot">RedDot Arena · append-only Ergebnis · Ausdruck zur Dokumentation</p>
  <script>window.onload = function () { setTimeout(function () { window.print(); }, 120); };</script>
</body>
</html>`;
}

/** Opens a print preview with target face + shot table (works in Tauri WebView2). */
export function printShotCard(input: ShotCardPrintInput): void {
  if (input.shots.length === 0) return;
  openPrintHtml(buildHtml(input));
}
