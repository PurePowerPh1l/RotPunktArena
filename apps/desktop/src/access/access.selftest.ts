/**
 * Access / capability selftest — no DOM.
 * Run: npm run test:access --workspace=@rotpunktarena/desktop
 */
import { adminAccessStore } from "./adminAccessStore.ts";
import {
  assertCapability,
  CapabilityDeniedError,
  hasCapability,
} from "./capabilities.ts";
import {
  isDeveloperEntryVisible,
  isDeveloperModeEnabled,
  resolveDeveloperAccessPolicy,
} from "./developerAccess.ts";
import { getAppAccessSnapshot } from "./appAccess.ts";
import type { AppAccessSnapshot, Capability } from "./types.ts";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

function snapshot(partial?: Partial<AppAccessSnapshot>): AppAccessSnapshot {
  return {
    adminAccessState: "unconfigured",
    isAdminModeEnabled: false,
    isDeveloperModeEnabled: true,
    developerAccessPolicy: "always-visible",
    ...partial,
  };
}

assert(
  resolveDeveloperAccessPolicy(true) === "always-visible",
  "dev → always-visible",
);
assert(resolveDeveloperAccessPolicy(false) === "disabled", "release → disabled");
assert(isDeveloperEntryVisible("always-visible"), "dev entry visible");
assert(isDeveloperModeEnabled("always-visible"), "developer mode on");
assert(!isDeveloperEntryVisible("disabled"), "disabled hides entry");
assert(!isDeveloperModeEnabled("disabled"), "disabled turns mode off");
assert(
  !isDeveloperEntryVisible("hidden-trigger"),
  "hidden-trigger hides until gesture",
);

const locked = snapshot();
const unlocked = snapshot({
  adminAccessState: "unlocked",
  isAdminModeEnabled: true,
});

const always: Capability[] = ["settings:open", "backup:create"];
for (const cap of always) {
  assert(hasCapability(cap, locked), `${cap} always allowed`);
}

assert(!hasCapability("backup:restore", locked), "restore locked without admin");
assert(hasCapability("backup:restore", unlocked), "restore with admin");
assert(!hasCapability("admin:reset", locked), "reset locked without admin");
assert(hasCapability("admin:reset", unlocked), "reset with admin");
assert(
  !hasCapability("admin:bureau-edit", locked),
  "bureau edit locked without admin",
);
assert(hasCapability("admin:test-unlock", locked), "test-unlock when dev on");
assert(
  !hasCapability("admin:test-unlock", snapshot({ isDeveloperModeEnabled: false })),
  "test-unlock denied when developer off",
);
assert(hasCapability("developer:diagnostics", locked), "diagnostics when dev on");
assert(
  !hasCapability(
    "developer:diagnostics",
    snapshot({ isDeveloperModeEnabled: false }),
  ),
  "diagnostics denied when developer off",
);

adminAccessStore.lock();
assert(
  String(adminAccessStore.state) === "unconfigured",
  "default/lock → unconfigured",
);
assert(!adminAccessStore.isAdminModeEnabled, "not enabled after lock");

adminAccessStore.enableAdminForTests();
assert(String(adminAccessStore.state) === "unlocked", "test unlock → unlocked");
assert(adminAccessStore.isAdminModeEnabled, "enabled after test unlock");

assertCapability("backup:restore", getAppAccessSnapshot());

adminAccessStore.lock();
let denied = false;
try {
  assertCapability("backup:restore", getAppAccessSnapshot());
} catch (e) {
  denied = e instanceof CapabilityDeniedError;
}
assert(denied, "restore assert fails after lock");

console.log("access selftest: ok");
