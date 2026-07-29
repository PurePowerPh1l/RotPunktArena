import type { UiShot } from "@rotpunktarena/domain";
import { type ScoreDisplayMode } from "../components/TargetFace";
import {
  fitFaceScale,
  scoringFaceChromeSvg,
  shotCoordsToSvg,
} from "../components/targetFaceGeometry";
import { bestShotOf } from "../hooks/useScoreDisplay";

/** Print mark radius — larger than live face so dots stay visible on paper. */
function printMarkRadius(emphasized: boolean): number {
  return emphasized ? 2.3 : 1.8;
}

/**
 * Scoring face + shot marks for HTML print.
 * Matches TargetFace contrast (brown fill + light stroke) so hits stay
 * readable on the black 10-ring when printed.
 */
export function printTargetSvg(
  shots: UiShot[],
  displayMode: ScoreDisplayMode,
  sizePx = 320,
): string {
  const scale = fitFaceScale(shots);
  const lastIdx = shots[shots.length - 1]?.shotIndex;
  const best = bestShotOf(shots, displayMode);
  const marks = shots
    .map((s) => {
      const { cx, cy } = shotCoordsToSvg(s.x, s.y);
      const active = s.shotIndex === lastIdx;
      const isBest = best != null && s.shotIndex === best.shotIndex;
      const hi = active || isBest;
      const fill = active ? "#c0392b" : isBest ? "#b8860b" : "#4a3228";
      const stroke = hi ? "#ffffff" : "#f5f0e6";
      const strokeW = hi ? 0.45 : 0.55;
      const r = printMarkRadius(hi);
      const label = String(s.shotIndex);
      const fontSize = 2.6;
      const charW = fontSize * 0.42;
      const padX = 0.55;
      const padY = 0.35;
      const boxW = Math.max(label.length, 1) * charW + padX * 2;
      const boxH = fontSize + padY * 2;
      const lx = cx + 2.4 + boxW / 2;
      const ly = cy - 1.6;
      return (
        `<g>` +
        `<circle cx="${cx}" cy="${cy}" r="${r}" fill="${fill}" stroke="${stroke}" stroke-width="${strokeW}"/>` +
        `<rect x="${lx - boxW / 2}" y="${ly - boxH / 2}" width="${boxW}" height="${boxH}" rx="0.6" fill="#fff" stroke="#333" stroke-width="0.25" opacity="0.92"/>` +
        `<text x="${lx}" y="${ly}" font-size="${fontSize}" fill="#222" font-family="Segoe UI,sans-serif" text-anchor="middle" dominant-baseline="central">${label}</text>` +
        `</g>`
      );
    })
    .join("");
  return `
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" width="${sizePx}" height="${sizePx}">
  <g transform="translate(50 50) scale(${scale}) translate(-50 -50)">
  ${scoringFaceChromeSvg()}
  ${marks}
  </g>
</svg>`;
}
