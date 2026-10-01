import type { LiveState } from "@rotpunktarena/domain";

/** Full projections replace deltas: newer snapshots also repair revision gaps. */
export function chooseLiveSnapshot(current: LiveState | null, incoming: LiveState): LiveState | null {
  if (!Number.isSafeInteger(incoming.revision) || incoming.revision < 1) return current;
  if (incoming.sessionId !== (incoming.session?.id ?? null)) return current;
  if (current && incoming.revision <= current.revision) return current;
  return incoming;
}
