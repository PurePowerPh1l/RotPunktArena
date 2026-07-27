/**
 * Access / capability selftest — no DOM.
 * Run: npm run test:access --workspace=@reddot/desktop
 */
import { adminAccessStore } from "./adminAccessStore.ts";
import {
  assertCapability,
  CapabilityDeniedError,
  hasCapability,
} from "./capabilities.ts";
import {
  DEVELOPER_ACCESS_POLICY,
  isDeveloperEntryVisible,
  isDeveloperModeEnabled,
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
    developerAccessPolicy: DEVELOPER_ACCESS_POLICY,
    ...partial,
  };
}

// --- Policy defaults (current stage) ---
assert(DEVELOPER_ACCESS_POLICY === "always-visible", "policy always-visible");
assert(isDeveloperEntryVisible(), "dev entry visible");
assert(isDeveloperModeEnabled(), "developer mode on for always-visible");
assert(!isDeveloperEntryVisible("disabled"), "disabled hides entry");
assert(!isDeveloperModeEnabled("disabled"), "disabled turns mode off");
assert(
  !isDeveloperEntryVisible("hidden-trigger"),
  "hidden-trigger hides until gesture (placeholder)",
);

// --- Capabilities ---
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

// --- Store: ephemeral unlock ---
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
