/**
 * Arena active-competition list + epoch sync.
 * Owns reload/stale-id clear only — selection (`competitionId`) stays parent-owned.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { Competition } from "@rotpunktarena/domain";
import * as api from "../api/commands";
import { createRequestSeq } from "../lib/requestSeq";

type Args = {
  arenaVisible: boolean;
  competitionsEpoch: number;
  /** Parent-owned Arena selection (active only). */
  competitionId: string;
  onCompetitionIdChange: (id: string) => void;
};

export function useArenaActiveCompetitions({
  arenaVisible,
  competitionsEpoch,
  competitionId,
  onCompetitionIdChange,
}: Args) {
  const [competitions, setCompetitions] = useState<Competition[]>([]);
  const [competitionsReady, setCompetitionsReady] = useState(false);
  /** Epoch value that the current `competitions` list belongs to. */
  const [listEpoch, setListEpoch] = useState(-1);
  const [loadError, setLoadError] = useState<string | null>(null);
  const requestSeq = useRef(createRequestSeq()).current;

  const reloadCompetitions = useCallback(async () => {
    const token = requestSeq.begin();
    const epochAtStart = competitionsEpoch;
    let list: Competition[];
    try {
      list = await api.listCompetitions();
    } catch (error) {
      if (requestSeq.isCurrent(token)) setLoadError(String(error));
      throw error;
    }
    if (!requestSeq.isCurrent(token)) return list;
    // Arena: only active competitions are selectable (drafts etc. stay in Verwaltung).
    const active = list.filter((c) => c.status === "active");
    setCompetitions(active);
    setCompetitionsReady(true);
    setListEpoch(epochAtStart);
    setLoadError(null);
    return list;
  }, [competitionsEpoch, requestSeq]);

  useEffect(() => {
    void reloadCompetitions().catch(() => {});
  }, [arenaVisible, reloadCompetitions]);

  // Clear only after the list matching competitionsEpoch has loaded (avoids
  // wiping a just-activated id while reload is still in flight).
  useEffect(() => {
    if (!competitionsReady || listEpoch !== competitionsEpoch) return;
    if (competitionId && !competitions.some((c) => c.id === competitionId)) {
      onCompetitionIdChange("");
    }
  }, [
    competitionsReady,
    listEpoch,
    competitionsEpoch,
    competitions,
    competitionId,
    onCompetitionIdChange,
  ]);

  return {
    competitions,
    competitionsReady,
    listEpoch,
    loadError,
    reloadCompetitions,
  };
}
