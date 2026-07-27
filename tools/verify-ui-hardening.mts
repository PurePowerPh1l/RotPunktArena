/**
 * Lightweight verification for P0–P3 UI mutation hardening.
 * Run: node --experimental-strip-types tools/verify-ui-hardening.mts
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequestSeq } from "../apps/desktop/src/lib/requestSeq.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
let failed = 0;

function assert(cond: boolean, msg: string) {
  if (!cond) {
    failed += 1;
    console.error(`FAIL  ${msg}`);
  } else {
    console.log(`OK    ${msg}`);
  }
}

type SortMode = "punkte" | "teiler";
type Entry = {
  entryId: string;
  startOrder: number;
  rankPunkte?: number | null;
  rankTeiler?: number | null;
};
type Team = {
  teamId: string;
  sortOrder: number;
  rankPunkte?: number | null;
  rankTeiler?: number | null;
};

/** Mirror of apps/desktop/src/lib/resultRank.ts (kept local to avoid React import). */
function entryRank(r: Entry, sortMode: SortMode): number | null {
  return (sortMode === "teiler" ? r.rankTeiler : r.rankPunkte) ?? null;
}
function teamRank(t: Team, sortMode: SortMode): number | null {
  return (sortMode === "teiler" ? t.rankTeiler : t.rankPunkte) ?? null;
}
function compareByEntryRank(a: Entry, b: Entry, sortMode: SortMode): number {
  const ra = entryRank(a, sortMode);
  const rb = entryRank(b, sortMode);
  if (ra == null && rb == null) return a.startOrder - b.startOrder;
  if (ra == null) return 1;
  if (rb == null) return -1;
  return ra - rb;
}
function compareByTeamRank(a: Team, b: Team, sortMode: SortMode): number {
  const ra = teamRank(a, sortMode);
  const rb = teamRank(b, sortMode);
  if (ra == null && rb == null) return a.sortOrder - b.sortOrder;
  if (ra == null) return 1;
  if (rb == null) return -1;
  return ra - rb;
}

// --- requestSeq ---
{
  const seq = createRequestSeq();
  const a = seq.begin();
  const b = seq.begin();
  assert(seq.isCurrent(b), "requestSeq: latest token is current");
  assert(!seq.isCurrent(a), "requestSeq: stale token is not current");
}

// --- resultRank ordering ---
{
  const a: Entry = { entryId: "a", startOrder: 1, rankPunkte: 2, rankTeiler: 1 };
  const b: Entry = { entryId: "b", startOrder: 2, rankPunkte: 1, rankTeiler: 2 };
  const none: Entry = { entryId: "n", startOrder: 3, rankPunkte: null, rankTeiler: null };

  assert(entryRank(a, "punkte") === 2, "entryRank punkte");
  assert(entryRank(a, "teiler") === 1, "entryRank teiler");
  assert(entryRank(none, "punkte") === null, "entryRank null without rank");
  assert(compareByEntryRank(b, a, "punkte") < 0, "compareByEntryRank orders by punkte rank");
  assert(compareByEntryRank(a, b, "teiler") < 0, "compareByEntryRank orders by teiler rank");
  assert(compareByEntryRank(none, a, "punkte") > 0, "unranked sorts after ranked");

  const t1: Team = { teamId: "t1", sortOrder: 0, rankPunkte: 2, rankTeiler: 1 };
  const t2: Team = { teamId: "t2", sortOrder: 1, rankPunkte: 1, rankTeiler: 2 };
  assert(teamRank(t2, "punkte") === 1, "teamRank punkte");
  assert(compareByTeamRank(t2, t1, "punkte") < 0, "compareByTeamRank orders by punkte");
}

// --- source text matches lib/resultRank.ts ---
{
  const lib = fs.readFileSync(
    path.join(root, "apps/desktop/src/lib/resultRank.ts"),
    "utf8",
  );
  assert(lib.includes("rankTeiler") && lib.includes("rankPunkte"), "resultRank.ts uses Rust ranks");
  assert(!lib.includes("punkteTotal -") && !lib.includes("teilerAvg -"), "resultRank.ts does not compare scores");
}

// --- static source checks ---
{
  const srcRoot = path.join(root, "apps/desktop/src");
  const banned = [
    "abandonSession",
    "exportEmergencyBundle",
    "IconBureau",
    "IconLive",
    "runCommand",
    "rankIndividuals",
    "compareBySortMode",
  ];
  const files: string[] = [];
  function walk(dir: string) {
    for (const name of fs.readdirSync(dir)) {
      const p = path.join(dir, name);
      const st = fs.statSync(p);
      if (st.isDirectory()) walk(p);
      else if (/\.(ts|tsx)$/.test(name)) files.push(p);
    }
  }
  walk(srcRoot);
  const hit: string[] = [];
  for (const file of files) {
    const text = fs.readFileSync(file, "utf8");
    for (const needle of banned) {
      if (text.includes(needle)) hit.push(`${path.relative(root, file)}:${needle}`);
    }
  }
  assert(hit.length === 0, `no banned symbols (${hit.join(", ") || "clean"})`);

  const checks: Array<[string, string]> = [
    ["apps/desktop/src/hooks/useAsyncAction.ts", "AsyncActionResult"],
    ["apps/desktop/src/hooks/useLiveSession.ts", "startEntryPrepared"],
    ["apps/desktop/src/hooks/useLiveSession.ts", "stopThenStartEntry"],
    ["apps/desktop/src/views/BureauView.tsx", "resultsEpoch"],
    ["apps/desktop/src/views/LiveStandView.tsx", "startEntryPrepared"],
    ["apps/desktop/src/views/bureau/PeoplePanel.tsx", "if (ok) resetForm()"],
    ["apps/desktop/src/views/bureau/CompetitionCreateForm.tsx", "if (!ok) return"],
    ["apps/desktop/src/views/bureau/TeamsPanel.tsx", "scheduleTeamCount"],
    ["apps/desktop/src/components/RecoveryGate.tsx", "useAsyncAction"],
    ["apps/desktop/src/components/DevPanel.tsx", "useAsyncAction"],
    ["docs/code-guidelines.md", "UI-Gamification"],
    ["packages/domain/src/index.ts", "rankPunkte"],
    ["apps/desktop/src-tauri/src/db/results.rs", "assign_entry_ranks"],
    ["apps/desktop/src-tauri/src/db/teams.rs", "assign_team_ranks"],
  ];

  for (const [rel, needle] of checks) {
    const text = fs.readFileSync(path.join(root, rel), "utf8");
    assert(text.includes(needle), `contains "${needle}" in ${rel}`);
  }

  // Bureau results effect must not depend on raw entries/teams arrays
  const bureau = fs.readFileSync(
    path.join(root, "apps/desktop/src/views/BureauView.tsx"),
    "utf8",
  );
  assert(
    !bureau.includes("[b.selectedId, b.entries, b.teams"),
    "BureauView does not reload results on every entries/teams identity change",
  );
  assert(
    bureau.includes("resultsEpoch"),
    "BureauView uses resultsEpoch for selective reload",
  );
}

if (failed > 0) {
  console.error(`\n${failed} check(s) failed`);
  process.exit(1);
}
console.log("\nAll verification checks passed.");
