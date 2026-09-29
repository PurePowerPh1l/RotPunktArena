import assert from "node:assert/strict";
import { build } from "esbuild";

// Bundle the actual production module, including its normal import graph.
const bundle = await build({
  entryPoints: ["apps/desktop/src/hooks/bureau/mutations.ts"],
  bundle: true, write: false, platform: "node", format: "esm",
});
const { createBureauMutations } = await import(
  `data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`
);

function fixture(refreshFails = true, writeFails = false) {
  const warnings: string[] = [];
  const errors: string[] = [];
  const calls: Array<{ name: string; args: unknown[] }> = [];
  const refresh = async () => { if (refreshFails) throw new Error("refresh failed"); };
  const api = new Proxy({}, {
    get: (_target, name: string) => async (...args: unknown[]) => {
      calls.push({ name, args });
      if (writeFails) throw new Error("write failed");
      return { id: "created", person: { id: "person" } };
    },
  });
  const mutations = createBureauMutations({
    mutate: async (write: () => Promise<void>) => {
      try { await write(); return true; }
      catch (error) { errors.push(String(error)); return false; }
    },
    onRefreshError: (message: string) => warnings.push(message),
    selectedId: "competition", peopleQuery: "",
    reloadPeople: refresh, reloadCompetitions: refresh,
    reloadEntries: refresh, reloadTeams: refresh,
    setSelectedId: () => {}, setEntries: () => {},
  }, api);
  return { mutations, warnings, errors, calls };
}

const input = { name: "Test", date: "2026-09-29", discipline: "Luftgewehr", maxShots: 10, scoringMode: "ringe", activateOnCreate: true };
const created = fixture();
assert.equal(await created.mutations.createCompetition(input), "created");
assert.deepEqual(created.calls, [{ name: "createCompetition", args: [input] }]);
assert.equal(created.errors.length, 0);
assert.match(created.warnings[0], /^Gespeichert\./);

for (const [name, args] of [
  ["createPerson", [{ firstName: "Test", lastName: "Starter" }]],
  ["createTeam", ["Team"]],
  ["saveAsTemplate", []],
  ["createFromTemplate", ["template"]],
  ["addShooterByName", ["Test Starter"]],
] as const) {
  const test = fixture();
  assert.equal(await test.mutations[name](...args), true, `${name}: refresh failure must preserve success`);
  assert.equal(test.errors.length, 0);
  assert.ok(test.warnings.length > 0);
}
const failed = fixture(false, true);
assert.equal(await failed.mutations.createCompetition(input), null);
assert.equal(failed.errors.length, 1);
assert.equal(failed.warnings.length, 0);
const success = fixture(false);
assert.equal(await success.mutations.createCompetition(input), "created");
assert.equal(success.warnings.length, 0);
console.log("Bureau mutation tests passed: committed writes survive refresh failures; write errors remain errors.");
