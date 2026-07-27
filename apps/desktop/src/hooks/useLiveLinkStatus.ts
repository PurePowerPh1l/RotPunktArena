import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionStatus } from "@reddot/domain";

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

/** App-lifetime RFCOMM link status for the global top-bar badge (not session). */
export function useLiveLinkStatus(): LiveLinkInfo {
  const [info, setInfo] = useState<Omit<LiveLinkInfo, "refresh">>({
    status: "disconnected",
    rfcommStatus: "idle",
    generation: 0,
    linked: false,
    hasTarget: false,
    rfcommFeature: false,
    needsSetup: false,
    refreshing: false,
  });

  const refresh = useCallback(async () => {
    try {
      const s = await invoke<RfcommStatusDto>("rfcomm_status");
      const targetName =
        s.target?.displayName ?? s.target?.btAddrHex ?? null;
      setInfo({
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
      setInfo({
        status: "disconnected",
        rfcommStatus: "idle",
        generation: 0,
        linked: false,
        hasTarget: false,
        rfcommFeature: false,
        needsSetup: false,
        refreshing: false,
      });
    }
  }, []);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 500);
    return () => window.clearInterval(id);
  }, [refresh]);

  return { ...info, refresh };
}
