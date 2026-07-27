/**
 * Pure live-start policy: hardware when RFCOMM/hardware link is available and link-ready;
 * simulator only when hardware link is unavailable or explicitly requested.
 * No silent simulator fallback for unknown live state.
 *
 * Naming note: LiveState.`serialFeature` / Rust `serial_feature` is a **legacy field name**.
 * Value is `cfg!(feature = "rfcomm")` — it means "native RFCOMM / hardware link compiled in",
 * not Virtual-COM / Cargo feature `serial`. Prefer reading it as hardware-link availability.
 */

export type SerialAvailability = "available" | "unavailable" | "unknown";

export type LiveStartDecision =
  | { action: "start"; useSimulator: boolean }
  | { action: "blocked"; reason: "link_required" | "state_unavailable" };

/**
 * Live-State → availability.
 * Parameter mirrors LiveState.`serialFeature` (legacy name = RFCOMM/hardware link flag).
 */
export function serialAvailabilityFromLive(
  serialFeature: boolean | undefined,
): SerialAvailability {
  if (serialFeature === true) return "available";
  if (serialFeature === false) return "unavailable";
  return "unknown";
}

/**
 * Implicit hardware start (main button / startNext).
 * - available + linkReady → hardware
 * - available + !linkReady → blocked
 * - unavailable → simulator
 * - unknown → blocked (never silent sim)
 */
export function resolveLiveHardwareStart(opts: {
  serial: SerialAvailability;
  linkReady: boolean;
}): LiveStartDecision {
  switch (opts.serial) {
    case "unknown":
      return { action: "blocked", reason: "state_unavailable" };
    case "available":
      if (!opts.linkReady) {
        return { action: "blocked", reason: "link_required" };
      }
      return { action: "start", useSimulator: false };
    case "unavailable":
      return { action: "start", useSimulator: true };
  }
}

/** Explicit simulator — no link gate, no state gate. */
export function resolveLiveExplicitSimulator(): {
  action: "start";
  useSimulator: true;
} {
  return { action: "start", useSimulator: true };
}

/** True iff the hardware resolver returns blocked. */
export function hardwareLiveStartBlocked(opts: {
  serial: SerialAvailability;
  linkReady: boolean;
}): boolean {
  return resolveLiveHardwareStart(opts).action === "blocked";
}
