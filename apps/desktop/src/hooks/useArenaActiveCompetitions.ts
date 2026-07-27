/**
 * Arena active-competition list + epoch sync.
 * Owns reload/stale-id clear only — selection (`competitionId`) stays parent-owned.
 */
import { useEffect, useState } from "react";
import type { Competition } from "@rotpunktarena/domain";
import * as api from "../api/commands";

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

  const reloadCompetitions = async () => {
    const epochAtStart = competitionsEpoch;
    const list = await api.listCompetitions();
    // Arena: only active competitions are selectable (drafts etc. stay in Verwaltung).
    const active = list.filter((c) => c.status === "active");
    setCompetitions(active);
    setCompetitionsReady(true);
    setListEpoch(epochAtStart);
    return list;
  };

  useEffect(() => {
    void reloadCompetitions();
  }, [arenaVisible, competitionsEpoch]);

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
    reloadCompetitions,
  };
}
