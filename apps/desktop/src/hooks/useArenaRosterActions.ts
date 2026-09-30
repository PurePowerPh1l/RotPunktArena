import { useState } from "react";
import type { ShooterValue } from "../components/ShooterAutocomplete";
import type { CompetitionCreateInput } from "../views/bureau/CompetitionCreateForm";
import { useCompetitionRoster } from "./useCompetitionRoster";
import { useArenaActiveCompetitions } from "./useArenaActiveCompetitions";
import * as api from "../api/commands";

type Roster = ReturnType<typeof useCompetitionRoster>;
type Active = ReturnType<typeof useArenaActiveCompetitions>;

type Args = {
  competitionId: string;
  teamScoringEnabled: boolean;
  teamId: string;
  teams: Roster["teams"];
  refreshTeams: Roster["refreshTeams"];
  refreshEntries: Roster["refreshEntries"];
  setEntryId: Roster["setEntryId"];
  setTeamId: Roster["setTeamId"];
  reloadCompetitions: Active["reloadCompetitions"];
  onCompetitionIdChange: (id: string) => void;
  onBureauCompetitionIdChange?: (id: string | null) => void;
  notify: (message: string) => void;
};

export function useArenaRosterActions({
  competitionId, teamScoringEnabled, teamId, teams, refreshTeams, refreshEntries,
  setEntryId, setTeamId, reloadCompetitions, onCompetitionIdChange,
  onBureauCompetitionIdChange, notify,
}: Args) {
  const [createBusy, setCreateBusy] = useState(false);
  const createCompetition = async (input: CompetitionCreateInput): Promise<boolean> => {
    if (createBusy) return false;
    setCreateBusy(true);
    try {
      const { activateOnCreate, ...createInput } = input;
      const created = await api.createCompetition(createInput);
      if (activateOnCreate) {
        try {
          await api.setCompetitionStatus(created.id, "active");
        } catch (e) {
          notify(
            `Wettkampf angelegt, aber Status konnte nicht auf Aktiv gesetzt werden: ${String(e)}`,
          );
          await reloadCompetitions();
          onBureauCompetitionIdChange?.(created.id);
          return true;
        }
        await reloadCompetitions();
        onCompetitionIdChange(created.id);
        onBureauCompetitionIdChange?.(created.id);
        return true;
      }
      await reloadCompetitions();
      // Draft stays Verwaltung-only; Arena keeps last active selection.
      onBureauCompetitionIdChange?.(created.id);
      return true;
    } catch (e) {
      notify(`Wettkampf konnte nicht angelegt werden: ${String(e)}`);
      return false;
    } finally {
      setCreateBusy(false);
    }
  };

  /** Add or select shooter as active starter; in team mode also assign to the team. */
  const ensureStarter = async (shooterValue: ShooterValue): Promise<boolean> => {
    if (!competitionId) return false;
    const trimmed = shooterValue.name.trim();
    if (!trimmed && !shooterValue.personId) return false;

    let personId = shooterValue.personId;
    if (!personId) {
      const promoted = await api.promoteTrainingShooter(trimmed);
      personId = promoted.person.id;
    }

    const list = await api.listEntries(competitionId);
    let entry = list.find((e) => e.personId === personId) ?? null;
    if (!entry) {
      try {
        entry = await api.addEntry(competitionId, personId);
      } catch (e) {
        const msg = String(e);
        if (msg.includes("bereits in der Startliste")) {
          const refreshed = await api.listEntries(competitionId);
          entry = refreshed.find((e) => e.personId === personId) ?? null;
        } else {
          throw e;
        }
      }
    }
    if (!entry) return false;

    if (teamScoringEnabled && teamId) {
      const team = teams.find((t) => t.id === teamId);
      if (team && !team.memberEntryIds.includes(entry.id)) {
        await api.addTeamMember(teamId, entry.id);
        await refreshTeams(competitionId);
      }
    }

    await refreshEntries(competitionId);
    setEntryId(entry.id);
    return true;
  };

  /** Create global team (or select if name already exists). */
  const ensureTeam = async (name: string): Promise<boolean> => {
    if (!teamScoringEnabled) return false;
    const trimmed = name.trim();
    if (!trimmed) return false;
    const existing = teams.find(
      (t) => t.name.trim().toLowerCase() === trimmed.toLowerCase(),
    );
    if (existing) {
      setTeamId(existing.id);
      return true;
    }
    try {
      const created = await api.createTeam(trimmed);
      await refreshTeams(competitionId || null);
      setTeamId(created.id);
      return true;
    } catch (e) {
      notify(`Team konnte nicht angelegt werden: ${String(e)}`);
      return false;
    }
  };

  return { createBusy, createCompetition, ensureStarter, ensureTeam };
}
