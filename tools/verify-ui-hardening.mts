/**
 * Lightweight verification for P0–P3 UI mutation hardening.
 * Run: node --experimental-strip-types tools/verify-ui-hardening.mts
 */
import { createRequestSeq } from "../apps/desktop/src/lib/requestSeq.ts";

let failed = 0;

function assert(cond: boolean, msg: string) {
  if (!cond) {
    failed += 1;
    console.error(`FAIL  ${msg}`);
  } else {
    console.log(`OK    ${msg}`);
  }
}

import { entryRank, teamRank, compareByEntryRank, compareByTeamRank } from "../apps/desktop/src/lib/resultRank.ts";
import type { EntryResultSummary as Entry, TeamResultSummary as Team } from "../packages/domain/src/index.ts";

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
  const a = { entryId: "a", startOrder: 1, rankPunkte: 2, rankTeiler: 1 } as Entry;
  const b = { entryId: "b", startOrder: 2, rankPunkte: 1, rankTeiler: 2 } as Entry;
  const none = { entryId: "n", startOrder: 3, rankPunkte: null, rankTeiler: null } as Entry;

  assert(entryRank(a, "punkte") === 2, "entryRank punkte");
  assert(entryRank(a, "teiler") === 1, "entryRank teiler");
  assert(entryRank(none, "punkte") === null, "entryRank null without rank");
  assert(compareByEntryRank(b, a, "punkte") < 0, "compareByEntryRank orders by punkte rank");
  assert(compareByEntryRank(a, b, "teiler") < 0, "compareByEntryRank orders by teiler rank");
  assert(compareByEntryRank(none, a, "punkte") > 0, "unranked sorts after ranked");

  const t1 = { teamId: "t1", sortOrder: 0, rankPunkte: 2, rankTeiler: 1 } as Team;
  const t2 = { teamId: "t2", sortOrder: 1, rankPunkte: 1, rankTeiler: 2 } as Team;
  assert(teamRank(t2, "punkte") === 1, "teamRank punkte");
  assert(compareByTeamRank(t2, t1, "punkte") < 0, "compareByTeamRank orders by punkte");
}

if (failed > 0) {
  console.error(`\n${failed} check(s) failed`);
  process.exit(1);
}
console.log("\nAll verification checks passed.");
