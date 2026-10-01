import type { LiveState } from "@rotpunktarena/domain";

/** Full projections replace deltas: newer snapshots also repair revision gaps. */
export function chooseLiveSnapshot(current: LiveState | null, incoming: LiveState): LiveState | null {
  if (incoming.contractVersion !== 1) return current;
  if (!Number.isSafeInteger(incoming.revision) || incoming.revision < 1) return current;
  if (!["idle", "probe", "match", "closed"].includes(incoming.phase)) return current;
  if (!["searching", "connected", "disconnected"].includes(incoming.status)) return current;
  if (!["simulator", "serial", "tcp", "rfcomm"].includes(incoming.transport)) return current;
  if (incoming.sessionId !== (incoming.session?.id ?? null)) return current;
  if (current && incoming.revision <= current.revision) return current;
  return incoming;
}
