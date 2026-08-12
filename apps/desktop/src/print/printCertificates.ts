import type {
  Competition,
  EntryResultSummary,
  TeamResultSummary,
} from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";
import { formatPersonName, formatScoreCompact } from "../lib/format";
import { entryRank, teamRank } from "../lib/resultRank";
import { escapeHtml, openPrintHtml } from "./printHtml";

export type CertificatesPrintInput = {
  competition: Competition;
  results: EntryResultSummary[];
  teamResults?: TeamResultSummary[];
  /** Ranking metric — same as ResultsPanel sort. */
  sortMode: ScoreDisplayMode;
  printedAt?: Date;
};

type CertificatePage = {
  place: number;
  title: string;
  subtitle: string;
  scoreLine: string;
  kindLabel: string;
};

function placeLabel(place: number): string {
  if (place === 1) return "1. Platz";
  if (place === 2) return "2. Platz";
  if (place === 3) return "3. Platz";
  return `${place}. Platz`;
}

function buildPages(input: CertificatesPrintInput): CertificatePage[] {
  const teiler = input.sortMode === "teiler";
  const pages: CertificatePage[] = [];

  const singles = input.results
    .filter((r) => {
      const rank = entryRank(r, input.sortMode);
      return rank != null && rank >= 1 && rank <= 3 && r.shotCount > 0;
    })
    .sort(
      (a, b) =>
        (entryRank(a, input.sortMode) ?? 99) -
        (entryRank(b, input.sortMode) ?? 99),
    );

  for (const r of singles) {
    const place = entryRank(r, input.sortMode)!;
    const name = formatPersonName(r.lastName, r.firstName, "—");
    const club = r.club?.trim() || "";
    const score = teiler
      ? `Ø Teiler ${formatScoreCompact(r.teilerAvg)}`
      : `${formatScoreCompact(r.punkteTotal)} Punkte`;
    pages.push({
      place,
      title: name,
      subtitle: club,
      scoreLine: score,
      kindLabel: "Einzelwertung",
    });
  }

  const teams = (input.teamResults ?? [])
    .filter((t) => {
      const rank = teamRank(t, input.sortMode);
      return rank != null && rank >= 1 && rank <= 3 && t.countingMembers > 0;
    })
    .sort(
      (a, b) =>
        (teamRank(a, input.sortMode) ?? 99) - (teamRank(b, input.sortMode) ?? 99),
    );

  for (const t of teams) {
    const place = teamRank(t, input.sortMode)!;
    const score = teiler
      ? `Σ Teiler ${formatScoreCompact(t.teilerSum)}`
      : `${formatScoreCompact(t.punkteTotal)} Punkte`;
    pages.push({
      place,
      title: t.name,
      subtitle: `${t.countingMembers} wertende Schützen`,
      scoreLine: score,
      kindLabel: "Teamwertung",
    });
  }

  return pages;
}

function buildHtml(input: CertificatesPrintInput, pages: CertificatePage[]): string {
  const when = (input.printedAt ?? new Date()).toLocaleString("de-DE");
  const compName = escapeHtml(input.competition.name);
  const discipline = escapeHtml(input.competition.discipline);
  const date = escapeHtml(input.competition.date);

  const sheets = pages
    .map((p) => {
      return `<article class="certificate">
  <p class="cert-eyebrow">${escapeHtml(p.kindLabel)}</p>
  <p class="cert-place">${escapeHtml(placeLabel(p.place))}</p>
  <h1 class="cert-name">${escapeHtml(p.title)}</h1>
  ${p.subtitle ? `<p class="cert-sub">${escapeHtml(p.subtitle)}</p>` : ""}
  <p class="cert-score">${escapeHtml(p.scoreLine)}</p>
  <div class="cert-event">
    <p class="cert-comp">${compName}</p>
    <p class="cert-meta">${discipline} · ${date}</p>
  </div>
  <p class="cert-foot">RotPunktArena · ausgedruckt ${escapeHtml(when)}</p>
</article>`;
    })
    .join("\n");

  return `<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8"/>
<title>Urkunden — ${compName}</title>
<style>
  @page { margin: 18mm; size: A4 portrait; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: "Segoe UI", "IBM Plex Sans", Georgia, serif;
    color: #1a1a18;
    background: #fff;
  }
  .noprint { margin: 1rem; }
  .certificate {
    min-height: calc(100vh - 36mm);
    padding: 2.5rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    page-break-after: always;
    border: 2px solid #2a2a28;
  }
  .certificate:last-child { page-break-after: auto; }
  .cert-eyebrow {
    margin: 0 0 0.75rem;
    font-size: 0.85rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #666;
    font-weight: 600;
  }
  .cert-place {
    margin: 0 0 1.25rem;
    font-size: 1.15rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-weight: 700;
    color: #8b3a1a;
  }
  .cert-name {
    margin: 0;
    font-size: 2.6rem;
    font-weight: 700;
    line-height: 1.15;
    max-width: 16ch;
  }
  .cert-sub {
    margin: 0.65rem 0 0;
    font-size: 1.1rem;
    color: #444;
  }
  .cert-score {
    margin: 1.75rem 0 0;
    font-size: 1.35rem;
    font-weight: 600;
  }
  .cert-event {
    margin-top: 2.5rem;
    padding-top: 1.25rem;
    border-top: 1px solid #ccc;
    width: min(100%, 28rem);
  }
  .cert-comp {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
  }
  .cert-meta {
    margin: 0.35rem 0 0;
    color: #555;
    font-size: 0.95rem;
  }
  .cert-foot {
    margin-top: 2.5rem;
    font-size: 0.75rem;
    color: #888;
  }
  @media print {
    .noprint { display: none !important; }
    .certificate {
      min-height: auto;
      height: calc(100vh - 36mm);
    }
  }
</style>
</head>
<body>
  ${sheets}
</body>
</html>`;
}

/** Opens print preview with one certificate page per podium place (single + teams). */
export function printCertificates(input: CertificatesPrintInput): void {
  const pages = buildPages(input);
  if (pages.length === 0) return;
  openPrintHtml(buildHtml(input, pages));
}

/** How many certificate pages would be printed (for UI enablement). */
export function countCertificatePages(input: CertificatesPrintInput): number {
  return buildPages(input).length;
}
