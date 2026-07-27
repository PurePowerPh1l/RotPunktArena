import type {
  Competition,
  EntryResultDetail,
  EntryResultSummary,
  EntryStatus,
  SeriesResultSummary,
  TeamResultSummary,
  TrainingSessionSummary,
  UiShot,
} from "@reddot/domain";
import { type ScoreDisplayMode } from "../components/TargetFace";
import {
  fitFaceScale,
  scoringFaceChromeSvg,
  shotCoordsToSvg,
} from "../components/targetFaceGeometry";
import { bestShotOf } from "../hooks/useScoreDisplay";
import { formatPersonName, formatScoreCompact } from "../lib/format";
import { ENTRY_LABEL } from "../views/bureau/labels";
import { escapeHtml, openPrintHtml, PRINT_BASE_CSS } from "./printHtml";

export type ResultsSheetInput = {
  competition: Competition;
  results: EntryResultSummary[];
  teamResults?: TeamResultSummary[];
  teilerMode: boolean;
  printedAt?: Date;
};

export type EntryResultSheetInput = {
  detail: EntryResultDetail;
  teilerMode: boolean;
  printedAt?: Date;
};

export type TrainingSheetInput = {
  title: string;
  filterLabel: string;
  sessions: TrainingSessionSummary[];
  printedAt?: Date;
};

function seriesLabel(s: SeriesResultSummary): string {
  if (s.isNachkauf || s.seriesIndex > 1) {
    return `Serie ${s.seriesIndex} (Nachkauf)`;
  }
  return `Serie ${s.seriesIndex}`;
}

function fmtShot(v: number): string {
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
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" width="220" height="220">
  <g transform="translate(50 50) scale(${scale}) translate(-50 -50)">
  ${scoringFaceChromeSvg()}
  ${marks}
  </g>
</svg>`;
}

function buildResultsHtml(input: ResultsSheetInput): string {
  const when = (input.printedAt ?? new Date()).toLocaleString("de-DE");
  const scoreHeader = input.teilerMode ? "Ø Teiler" : "Σ Punkte";
  const bestOfNote = input.competition.nachkaufEnabled
    ? " · Einzelwertung: beste Serie"
    : "";
  const rows = input.results
    .map((r) => {
      const name = escapeHtml(formatPersonName(r.lastName, r.firstName, ""));
      const status = escapeHtml(ENTRY_LABEL[r.status as EntryStatus] ?? r.status);
      const shots =
        input.competition.maxShots > 0
          ? `${r.shotCount} / ${input.competition.maxShots}`
          : String(r.shotCount);
      const score =
        r.shotCount === 0
          ? "—"
          : escapeHtml(
              formatScoreCompact(
                input.teilerMode ? r.teilerAvg : r.punkteTotal,
              ),
            );
      return `<tr><td>${r.startOrder}</td><td>${name}</td><td>${status}</td><td>${shots}</td><td>${score}</td></tr>`;
    })
    .join("");

  const teamBlock =
    input.teamResults && input.teamResults.length > 0
      ? `<h2>Teams</h2>
<table>
  <thead><tr><th>Rang</th><th>Team</th><th>${input.teilerMode ? "Σ Teiler" : "Σ Punkte"}</th></tr></thead>
  <tbody>${input.teamResults
    .map((t, i) => {
      const score =
        t.countingMembers === 0
          ? "—"
          : escapeHtml(
              formatScoreCompact(input.teilerMode ? t.teilerSum : t.punkteTotal),
            );
      return `<tr><td>${t.countingMembers > 0 ? i + 1 : "—"}</td><td>${escapeHtml(t.name)}</td><td>${score}</td></tr>`;
    })
    .join("")}</tbody>
</table>`
      : "";

  return `<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8"/>
<title>Ergebnisse — ${escapeHtml(input.competition.name)}</title>
<style>${PRINT_BASE_CSS}</style>
</head>
<body>
  <button class="noprint" onclick="window.print()" style="margin-bottom:1rem;padding:0.5rem 0.9rem;">Drucken</button>
  <h1>${escapeHtml(input.competition.name)}</h1>
  <p class="meta">${escapeHtml(input.competition.date)} · ${escapeHtml(input.competition.discipline)}${bestOfNote} · ${escapeHtml(when)}</p>
  <table>
    <thead><tr><th>#</th><th>Name</th><th>Status</th><th>Schüsse</th><th>${scoreHeader}</th></tr></thead>
    <tbody>${rows || "<tr><td colspan='5'>Keine Starter</td></tr>"}</tbody>
  </table>
  ${teamBlock}
  <p class="foot">RedDot Arena · Ergebnisprotokoll${bestOfNote ? " · Werte = beste Serie" : ""}</p>
  <script>window.onload = function () { setTimeout(function () { window.print(); }, 120); };</script>
</body>
</html>`;
}

export function printResultsSheet(input: ResultsSheetInput): void {
  openPrintHtml(buildResultsHtml(input));
}

function resolveSeriesList(detail: EntryResultDetail): SeriesResultSummary[] {
  if (detail.series && detail.series.length > 0) return detail.series;
  if (detail.shots.length === 0 && detail.summary.shotCount === 0) return [];
  return [
    {
      sessionId: detail.summary.sessionId ?? "best",
      seriesIndex: 1,
      endedAt: detail.summary.sessionEndedAt,
      shotCount: detail.summary.shotCount,
      punkteTotal: detail.summary.punkteTotal,
      teilerSum: detail.summary.teilerSum,
      teilerAvg: detail.summary.teilerAvg,
      isBest: true,
      isNachkauf: false,
      shots: detail.shots,
    },
  ];
}

function buildEntryResultHtml(input: EntryResultSheetInput): string {
  const { detail, teilerMode } = input;
  const when = (input.printedAt ?? new Date()).toLocaleString("de-DE");
  const name = formatPersonName(detail.summary.lastName, detail.summary.firstName);
  const seriesList = resolveSeriesList(detail);
  const overviewRows = seriesList
    .map((s) => {
      const label = escapeHtml(seriesLabel(s));
      const best = s.isBest ? " <strong>(Beste)</strong>" : "";
      const shots =
        detail.maxShots > 0
          ? `${s.shotCount} / ${detail.maxShots}`
          : String(s.shotCount);
      const punkte =
        s.shotCount === 0 ? "—" : escapeHtml(formatScoreCompact(s.punkteTotal));
      const teiler =
        s.shotCount === 0 ? "—" : escapeHtml(formatScoreCompact(s.teilerAvg));
      const cls = s.isBest ? ' class="best"' : "";
      return `<tr${cls}><td>${label}${best}</td><td>${shots}</td><td>${punkte}</td><td>${teiler}</td></tr>`;
    })
    .join("");

  const seriesBlocks = seriesList
    .map((s) => {
      const shots =
        s.shots && s.shots.length > 0
          ? s.shots
          : s.isBest
            ? detail.shots
            : [];
      if (shots.length === 0) {
        return `<section class="series-block${s.isBest ? " best" : ""}">
  <h2>${escapeHtml(seriesLabel(s))}${s.isBest ? " — Beste Serie" : ""}</h2>
  <p class="meta">Keine Schüsse</p>
</section>`;
      }
      const total = teilerMode
        ? (shots[shots.length - 1]?.seriesTeilerTotal ?? s.teilerSum)
        : s.punkteTotal;
      const rows = shots
        .map(
          (shot) =>
            `<tr><td>${shot.shotIndex}</td><td>${fmtShot(shot.valueDisplay)}</td><td>${fmtShot(shot.distanceDisplay)}</td><td>${fmtShot(shot.seriesTotal)}</td></tr>`,
        )
        .join("");
      return `<section class="series-block${s.isBest ? " best" : ""}">
  <h2>${escapeHtml(seriesLabel(s))}${s.isBest ? " — Beste Serie" : ""}</h2>
  <div class="grid">
    <div>${markSvg(shots, teilerMode ? "teiler" : "punkte")}</div>
    <div>
      <table>
        <thead><tr><th>#</th><th>Wert</th><th>Teiler</th><th>Σ</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
      <p class="total">${teilerMode ? "Σ Teiler" : "Σ Punkte"} <strong>${escapeHtml(formatScoreCompact(total))}</strong></p>
    </div>
  </div>
</section>`;
    })
    .join("");

  return `<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8"/>
<title>Ergebnis — ${escapeHtml(name)}</title>
<style>
${PRINT_BASE_CSS}
  tr.best td { font-weight: 700; }
  .series-block { margin-top: 1.25rem; page-break-inside: avoid; }
  .series-block.best h2 { color: #8a3a1a; }
  .grid {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 1rem;
    align-items: start;
  }
  .total { margin-top: 0.75rem; font-size: 1.05rem; }
  .total strong { font-size: 1.25rem; }
</style>
</head>
<body>
  <button class="noprint" onclick="window.print()" style="margin-bottom:1rem;padding:0.5rem 0.9rem;">Drucken</button>
  <h1>${escapeHtml(name)}</h1>
  <p class="meta">${escapeHtml(detail.competitionName)} · ${seriesList.length > 1 ? "alle Serien, beste markiert" : "Serie"} · ${escapeHtml(when)}</p>
  ${
    seriesList.length > 1
      ? `<h2>Übersicht</h2>
<table>
  <thead><tr><th>Serie</th><th>Schüsse</th><th>Σ Punkte</th><th>Ø Teiler</th></tr></thead>
  <tbody>${overviewRows}</tbody>
</table>`
      : ""
  }
  ${seriesBlocks || "<p>Keine Serien</p>"}
  <p class="foot">RedDot Arena · Ergebnis · ${seriesList.length > 1 ? "Wertung = beste Serie" : "Ausdruck zur Dokumentation"}</p>
  <script>window.onload = function () { setTimeout(function () { window.print(); }, 120); };</script>
</body>
</html>`;
}

/** Print all series for one entry; best series marked. */
export function printEntryResultSheet(input: EntryResultSheetInput): void {
  const series = resolveSeriesList(input.detail);
  const hasShots = series.some(
    (s) =>
      (s.shots && s.shots.length > 0) ||
      (s.isBest && input.detail.shots.length > 0) ||
      s.shotCount > 0,
  );
  if (!hasShots && input.detail.shots.length === 0) return;
  openPrintHtml(buildEntryResultHtml(input));
}

function buildTrainingHtml(input: TrainingSheetInput): string {
  const when = (input.printedAt ?? new Date()).toLocaleString("de-DE");
  const rows = [...input.sessions]
    .reverse()
    .map((s) => {
      const perShot = s.shotCount > 0 ? s.punkteTotal / s.shotCount : 0;
      return `<tr>
        <td>${escapeHtml(new Date(s.endedAt).toLocaleString("de-DE"))}</td>
        <td>${escapeHtml(s.shooterName)}</td>
        <td>${s.shotCount}</td>
        <td>${escapeHtml(formatScoreCompact(s.punkteTotal))}</td>
        <td>${escapeHtml(formatScoreCompact(perShot))}</td>
        <td>${escapeHtml(formatScoreCompact(s.teilerAvg))}</td>
      </tr>`;
    })
    .join("");

  return `<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8"/>
<title>${escapeHtml(input.title)}</title>
<style>${PRINT_BASE_CSS}</style>
</head>
<body>
  <button class="noprint" onclick="window.print()" style="margin-bottom:1rem;padding:0.5rem 0.9rem;">Drucken</button>
  <h1>${escapeHtml(input.title)}</h1>
  <p class="meta">${escapeHtml(input.filterLabel)} · ${escapeHtml(when)}</p>
  <table>
    <thead><tr><th>Datum</th><th>Schütze</th><th>Schüsse</th><th>Σ Punkte</th><th>Ø / Schuss</th><th>Ø Teiler</th></tr></thead>
    <tbody>${rows || "<tr><td colspan='6'>Keine Serien</td></tr>"}</tbody>
  </table>
  <p class="foot">RedDot Arena · Trainingshistorie</p>
  <script>window.onload = function () { setTimeout(function () { window.print(); }, 120); };</script>
</body>
</html>`;
}

export function printTrainingHistorySheet(input: TrainingSheetInput): void {
  openPrintHtml(buildTrainingHtml(input));
}
