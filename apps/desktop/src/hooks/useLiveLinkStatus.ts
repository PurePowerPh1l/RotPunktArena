import { useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionStatus } from "@rotpunktarena/domain";

type RfcommStatusDto = {
  status: string;
  reason: string;
  connectPhase?: string;
  connectOrigin?: string | null;
  generation: number;
  target: { displayName?: string; btAddrHex?: string } | null;
  rfcommFeature: boolean;
  needsSetup: boolean;
};

export type LiveLinkInfo = {
  status: ConnectionStatus;
  rfcommStatus: string;
  connectPhase?: string;
  connectOrigin?: string | null;
  generation: number;
  targetName?: string | null;
  reason?: string;
  linked: boolean;
  hasTarget: boolean;
  rfcommFeature: boolean;
  needsSetup: boolean;
  refreshing: boolean;
  refresh: () => Promise<void>;
};

type LinkSnapshot = Omit<LiveLinkInfo, "refresh">;

function mapRfcomm(status: string): ConnectionStatus {
  switch (status) {
    case "linked":
      return "connected";
    case "connecting":
    // Legacy-tolerant mapping; backend owner no longer emits reconnecting;
    // remove after compatibility window or backend event-contract audit.
    case "reconnecting":
    case "discovering":
      return "searching";
    default:
      return "disconnected";
  }
}

const DISCONNECTED: LinkSnapshot = {
  status: "disconnected",
  rfcommStatus: "idle",
  generation: 0,
  linked: false,
  hasTarget: false,
  rfcommFeature: false,
  needsSetup: false,
  refreshing: false,
};

const POLL_INTERVAL_MS = 500;

// One module-level store + poller shared by every hook consumer
// (badge, arena, settings) instead of one interval per mount.
let snapshot: LinkSnapshot = DISCONNECTED;
const listeners = new Set<() => void>();
let pollId: number | null = null;
let inFlight: Promise<void> | null = null;

function snapshotsEqual(a: LinkSnapshot, b: LinkSnapshot): boolean {
  return (
    a.status === b.status &&
    a.rfcommStatus === b.rfcommStatus &&
    a.connectPhase === b.connectPhase &&
    a.connectOrigin === b.connectOrigin &&
    a.generation === b.generation &&
    a.targetName === b.targetName &&
    a.reason === b.reason &&
    a.linked === b.linked &&
    a.hasTarget === b.hasTarget &&
    a.rfcommFeature === b.rfcommFeature &&
    a.needsSetup === b.needsSetup &&
    a.refreshing === b.refreshing
  );
}

function publish(next: LinkSnapshot) {
  if (snapshotsEqual(snapshot, next)) return;
  snapshot = next;
  listeners.forEach((l) => l());
}

async function fetchOnce(): Promise<void> {
  try {
    const s = await invoke<RfcommStatusDto>("rfcomm_status");
    const targetName = s.target?.displayName ?? s.target?.btAddrHex ?? null;
    publish({
      status: mapRfcomm(s.status),
      rfcommStatus: s.status,
      connectPhase: s.connectPhase,
      connectOrigin: s.connectOrigin ?? null,
      generation: s.generation,
      targetName,
      reason: s.reason || undefined,
      linked: s.status === "linked",
      hasTarget: Boolean(s.target),
      rfcommFeature: Boolean(s.rfcommFeature),
      needsSetup: Boolean(s.needsSetup) && Boolean(s.rfcommFeature),
      refreshing: false,
    });
  } catch {
    publish(DISCONNECTED);
  }
}

/** Deduped refresh — concurrent callers share one invoke. */
function refreshShared(): Promise<void> {
  if (!inFlight) {
    inFlight = fetchOnce().finally(() => {
      inFlight = null;
    });
  }
  return inFlight;
}

function subscribe(onChange: () => void): () => void {
  listeners.add(onChange);
  if (listeners.size === 1) {
    void refreshShared();
    pollId = window.setInterval(() => void refreshShared(), POLL_INTERVAL_MS);
  }
  return () => {
    listeners.delete(onChange);
    if (listeners.size === 0 && pollId !== null) {
      window.clearInterval(pollId);
      pollId = null;
    }
  };
}

function getSnapshot(): LinkSnapshot {
  return snapshot;
}

/** App-lifetime RFCOMM link status for the global top-bar badge (not session). */
export function useLiveLinkStatus(): LiveLinkInfo {
  const info = useSyncExternalStore(subscribe, getSnapshot);
  return { ...info, refresh: refreshShared };
}
