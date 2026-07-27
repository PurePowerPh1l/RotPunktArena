/**
 * Competition roster: entries/teams lists, selection ids, refresh + race seq.
 * Data/selection only — no handoff, mode, mutations, or start flow.
 */
import { useEffect, useRef, useState } from "react";
import type {
  Competition,
  CompetitionEntry,
  CompetitionTeam,
} from "@rotpunktarena/domain";
import * as api from "../api/commands";
import { createRequestSeq } from "../lib/requestSeq";

type Args = {
  competitionId: string;
  competitions: Competition[];
  /** Session running — triggers roster refresh (existing contract). */
  running: boolean;
};

export function useCompetitionRoster({
  competitionId,
  competitions,
  running,
}: Args) {
  const [entries, setEntries] = useState<CompetitionEntry[]>([]);
  const [entryId, setEntryId] = useState("");
  const [teams, setTeams] = useState<CompetitionTeam[]>([]);
  const [teamId, setTeamId] = useState("");
  const entriesSeq = useRef(createRequestSeq()).current;
  const teamsSeq = useRef(createRequestSeq()).current;

  const refreshEntries = async (compId: string) => {
    const token = entriesSeq.begin();
    const list = await api.listEntries(compId);
    if (!entriesSeq.isCurrent(token)) return list;
    setEntries(list);
    setEntryId((prev) => {
      if (prev && list.some((e) => e.id === prev)) return prev;
      const active = list.find((e) => e.status === "active");
      if (active) return active.id;
      const next =
        list.find((e) => e.status === "waiting" || e.status === "probe") ??
        list[0];
      return next?.id ?? "";
    });
    return list;
  };

  const refreshTeams = async (compId: string | null) => {
    const token = teamsSeq.begin();
    const list = await api.listTeams(compId);
    if (!teamsSeq.isCurrent(token)) return list;
    setTeams(list);
    setTeamId((prev) => {
      if (prev && list.some((t) => t.id === prev)) return prev;
      return list[0]?.id ?? "";
    });
    return list;
  };

  useEffect(() => {
    if (!competitionId) {
      entriesSeq.begin();
      teamsSeq.begin();
      setEntries([]);
      setEntryId("");
      setTeams([]);
      setTeamId("");
      return;
    }
    void refreshEntries(competitionId);
  }, [competitionId, running, entriesSeq]);

  const teamScoringEnabled = Boolean(
    competitions.find((c) => c.id === competitionId)?.teamScoringEnabled,
  );

  useEffect(() => {
    if (!teamScoringEnabled) {
      teamsSeq.begin();
      setTeams([]);
      setTeamId("");
      return;
    }
    // Global teams; resolve entry membership for the current competition when set.
    void refreshTeams(competitionId || null);
  }, [competitionId, teamScoringEnabled, running, teamsSeq]);

  // When team changes, keep entry only if they belong to the team.
  useEffect(() => {
    if (!teamScoringEnabled || !teamId) return;
    const team = teams.find((t) => t.id === teamId);
    if (!team) return;
    if (entryId && team.memberEntryIds.includes(entryId)) return;
    const next =
      team.memberEntryIds
        .map((id) => entries.find((e) => e.id === id))
        .find((e) => e && (e.status === "waiting" || e.status === "probe")) ??
      team.memberEntryIds
        .map((id) => entries.find((e) => e.id === id))
        .find((e) => e && e.status === "active") ??
      team.memberEntryIds.map((id) => entries.find((e) => e.id === id)).find(Boolean);
    setEntryId(next?.id ?? "");
  }, [teamId, teams, entries, entryId, teamScoringEnabled]);

  const selectedComp = competitions.find((c) => c.id === competitionId);
  const selectedEntry = entries.find((e) => e.id === entryId);
  const nachkaufEnabled = Boolean(selectedComp?.nachkaufEnabled);

  return {
    entries,
    entryId,
    setEntryId,
    teams,
    teamId,
    setTeamId,
    refreshEntries,
    refreshTeams,
    teamScoringEnabled,
    selectedComp,
    selectedEntry,
    nachkaufEnabled,
  };
}
