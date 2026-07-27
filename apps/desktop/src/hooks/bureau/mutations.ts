import type {
  CompetitionEntry,
  CompetitionStatus,
  EntryStatus,
  ScoringMode,
} from "@rotpunktarena/domain";
import * as api from "../../api/commands";

export type BureauMutate = (fn: () => Promise<void>) => Promise<boolean>;

export type BureauMutationCtx = {
  mutate: BureauMutate;
  peopleQuery: string;
  selectedId: string | null;
  reloadPeople: (query?: string) => Promise<void>;
  reloadCompetitions: () => Promise<void>;
  reloadEntries: (competitionId: string) => Promise<void>;
  reloadTeams: (competitionId?: string | null) => Promise<void>;
  setSelectedId: (
    id: string | null | ((prev: string | null) => string | null),
  ) => void;
  setEntries: (entries: CompetitionEntry[]) => void;
};

export function createBureauMutations(ctx: BureauMutationCtx) {
  const {
    mutate,
    peopleQuery,
    selectedId,
    reloadPeople,
    reloadCompetitions,
    reloadEntries,
    reloadTeams,
    setSelectedId,
    setEntries,
  } = ctx;

  const reloadTeamsForSelection = () => reloadTeams(selectedId);

  const createPerson = async (input: {
    firstName: string;
    lastName: string;
    club?: string;
  }): Promise<boolean> =>
    mutate(async () => {
      await api.createPerson(input);
      await reloadPeople(peopleQuery);
    });

  const updatePerson = async (
    personId: string,
    input: {
      firstName: string;
      lastName: string;
      club?: string;
    },
  ): Promise<boolean> =>
    mutate(async () => {
      await api.updatePerson(personId, input);
      await reloadPeople(peopleQuery);
      if (selectedId) await reloadEntries(selectedId);
    });

  const deletePerson = async (personId: string): Promise<boolean> =>
    mutate(async () => {
      await api.deletePerson(personId);
      await reloadPeople(peopleQuery);
      await reloadTeamsForSelection();
      if (selectedId) await reloadEntries(selectedId);
    });

  const setPersonArchived = async (
    personId: string,
    archived: boolean,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.setPersonArchived(personId, archived);
      await reloadPeople(peopleQuery);
    });

  const createCompetition = async (input: {
    name: string;
    date: string;
    discipline: string;
    maxShots: number;
    scoringMode: ScoringMode;
    nachkaufEnabled?: boolean;
    nachkaufShots?: number;
    teamScoringEnabled?: boolean;
    teamCount?: number;
    kind?: import("@rotpunktarena/domain").CompetitionKind;
    activateOnCreate?: boolean;
  }): Promise<string | null> => {
    let createdId: string | null = null;
    const ok = await mutate(async () => {
      const { activateOnCreate, ...createInput } = input;
      const c = await api.createCompetition(createInput);
      if (activateOnCreate) {
        await api.setCompetitionStatus(c.id, "active");
      }
      await reloadCompetitions();
      setSelectedId(c.id);
      createdId = c.id;
    });
    return ok ? createdId : null;
  };

  const updateCompetition = async (
    id: string,
    input: {
      name: string;
      date: string;
      discipline: string;
      maxShots: number;
      scoringMode: ScoringMode;
      nachkaufEnabled?: boolean;
      nachkaufShots?: number;
      teamScoringEnabled?: boolean;
      teamCount?: number;
      kind?: import("@rotpunktarena/domain").CompetitionKind;
      activateOnCreate?: boolean;
    },
  ): Promise<boolean> =>
    mutate(async () => {
      const { activateOnCreate: _activate, ...updateInput } = input;
      await api.updateCompetition(id, updateInput);
      await reloadCompetitions();
      await reloadTeams(id);
    });

  const saveAsTemplate = async (sourceId?: string): Promise<boolean> => {
    const id = sourceId ?? selectedId;
    if (!id) return false;
    return mutate(async () => {
      const c = await api.createFromCompetition({
        sourceId: id,
        asTemplate: true,
        copyEntries: true,
      });
      await reloadCompetitions();
      setSelectedId(c.id);
    });
  };

  const createFromTemplate = async (
    sourceId: string,
    date?: string,
  ): Promise<boolean> =>
    mutate(async () => {
      const c = await api.createFromCompetition({
        sourceId,
        date: date ?? new Date().toISOString().slice(0, 10),
        asTemplate: false,
        copyEntries: true,
      });
      await reloadCompetitions();
      setSelectedId(c.id);
    });

  const setStatus = async (status: CompetitionStatus): Promise<boolean> => {
    if (!selectedId) return false;
    return mutate(async () => {
      await api.setCompetitionStatus(selectedId, status);
      await reloadCompetitions();
    });
  };

  const setTeamSettings = async (
    teamScoringEnabled: boolean,
    teamCount: number,
  ): Promise<boolean> => {
    if (!selectedId) return false;
    return mutate(async () => {
      await api.setCompetitionTeamSettings(
        selectedId,
        teamScoringEnabled,
        teamCount,
      );
      await reloadCompetitions();
      await reloadTeams(selectedId);
    });
  };

  const addPersonToStartList = async (personId: string): Promise<boolean> => {
    if (!selectedId) return false;
    return mutate(async () => {
      await api.addEntry(selectedId, personId);
      await reloadEntries(selectedId);
      await reloadTeams(selectedId);
    });
  };

  /** Freitext → Person (Büro) + Startlisten-Eintrag; verknüpft auch Trainingsserien. */
  const addShooterByName = async (name: string): Promise<boolean> => {
    if (!selectedId) return false;
    const trimmed = name.trim();
    if (!trimmed) return false;
    return mutate(async () => {
      const promoted = await api.promoteTrainingShooter(trimmed);
      await api.addEntry(selectedId, promoted.person.id);
      await Promise.all([
        reloadPeople(peopleQuery),
        reloadEntries(selectedId),
        reloadTeams(selectedId),
      ]);
    });
  };

  const reorderEntries = async (entryIds: string[]): Promise<boolean> => {
    if (!selectedId) return false;
    return mutate(async () => {
      try {
        const list = await api.reorderEntries(selectedId, entryIds);
        setEntries(list);
      } catch (e) {
        await reloadEntries(selectedId);
        throw e;
      }
    });
  };

  const setEntryStatus = async (
    entryId: string,
    status: EntryStatus,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.setEntryStatus(entryId, status);
      if (selectedId) await reloadEntries(selectedId);
    });

  const removeEntry = async (entryId: string): Promise<boolean> =>
    mutate(async () => {
      await api.removeEntry(entryId);
      if (selectedId) {
        await Promise.all([reloadEntries(selectedId), reloadTeams(selectedId)]);
      }
    });

  const cloneFrom = async (fromCompetitionId: string): Promise<boolean> => {
    if (!selectedId) return false;
    return mutate(async () => {
      await api.cloneEntries(fromCompetitionId, selectedId);
      await Promise.all([reloadEntries(selectedId), reloadTeams(selectedId)]);
    });
  };

  const createTeam = async (name: string): Promise<boolean> =>
    mutate(async () => {
      await api.createTeam(name);
      await reloadTeamsForSelection();
    });

  const renameTeam = async (teamId: string, name: string): Promise<boolean> =>
    mutate(async () => {
      await api.renameTeam(teamId, name);
      await reloadTeamsForSelection();
    });

  const setTeamArchived = async (
    teamId: string,
    archived: boolean,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.setTeamArchived(teamId, archived);
      await reloadTeamsForSelection();
    });

  const removeTeam = async (teamId: string): Promise<boolean> =>
    mutate(async () => {
      await api.removeTeam(teamId);
      await reloadTeamsForSelection();
    });

  const addTeamMember = async (
    teamId: string,
    entryId: string,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.addTeamMember(teamId, entryId);
      await reloadTeamsForSelection();
    });

  const removeTeamMember = async (
    teamId: string,
    entryId: string,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.removeTeamMember(teamId, entryId);
      await reloadTeamsForSelection();
    });

  const addTeamPerson = async (
    teamId: string,
    personId: string,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.addTeamPerson(teamId, personId);
      await reloadTeamsForSelection();
    });

  const removeTeamPerson = async (
    teamId: string,
    personId: string,
  ): Promise<boolean> =>
    mutate(async () => {
      await api.removeTeamPerson(teamId, personId);
      await reloadTeamsForSelection();
    });

  return {
    createPerson,
    updatePerson,
    deletePerson,
    setPersonArchived,
    createCompetition,
    updateCompetition,
    saveAsTemplate,
    createFromTemplate,
    setStatus,
    setTeamSettings,
    addPersonToStartList,
    addShooterByName,
    reorderEntries,
    setEntryStatus,
    removeEntry,
    cloneFrom,
    createTeam,
    renameTeam,
    setTeamArchived,
    removeTeam,
    addTeamMember,
    removeTeamMember,
    addTeamPerson,
    removeTeamPerson,
  };
}
