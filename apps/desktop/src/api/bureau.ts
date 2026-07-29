import { invoke } from "@tauri-apps/api/core";
import type {
  Competition,
  CompetitionEntry,
  CompetitionStatus,
  CompetitionTeam,
  EntryStatus,
  Person,
  ScoringMode,
  TeamResultSummary,
} from "@rotpunktarena/domain";

export async function listPeople(
  query?: string,
  includeArchived = false,
): Promise<Person[]> {
  return invoke("list_people", {
    query: query ?? null,
    includeArchived,
  });
}

export async function createPerson(input: {
  firstName: string;
  lastName: string;
  club?: string | null;
}): Promise<Person> {
  return invoke("create_person", { person: input });
}

export async function updatePerson(
  id: string,
  input: {
    firstName: string;
    lastName: string;
    club?: string | null;
  },
): Promise<Person> {
  return invoke("update_person", { id, person: input });
}

export async function deletePerson(id: string): Promise<void> {
  await invoke("delete_person", { id });
}

export async function setPersonArchived(
  id: string,
  archived: boolean,
): Promise<Person> {
  return invoke("set_person_archived", { id, archived });
}

export async function listCompetitions(
  includeArchived = false,
): Promise<Competition[]> {
  return invoke("list_competitions", { includeArchived });
}

export async function createCompetition(input: {
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
  tenthsEnabled?: boolean;
  probeEnabled?: boolean;
}): Promise<Competition> {
  return invoke("create_competition", { competition: input });
}

export async function updateCompetition(
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
    tenthsEnabled?: boolean;
    probeEnabled?: boolean;
  },
): Promise<Competition> {
  return invoke("update_competition", { id, competition: input });
}

export async function createFromCompetition(input: {
  sourceId: string;
  name?: string | null;
  date?: string | null;
  asTemplate?: boolean;
  copyEntries?: boolean;
}): Promise<Competition> {
  return invoke("create_from_competition", {
    sourceId: input.sourceId,
    name: input.name ?? null,
    date: input.date ?? null,
    asTemplate: input.asTemplate ?? false,
    copyEntries: input.copyEntries ?? true,
  });
}

export async function setCompetitionStatus(
  id: string,
  status: CompetitionStatus,
): Promise<Competition> {
  return invoke("set_competition_status", { id, status });
}

export async function setCompetitionTeamSettings(
  id: string,
  teamScoringEnabled: boolean,
  teamCount: number,
): Promise<Competition> {
  return invoke("set_competition_team_settings", {
    id,
    teamScoringEnabled,
    teamCount,
  });
}

export async function listEntries(competitionId: string): Promise<CompetitionEntry[]> {
  return invoke("list_entries", { competitionId });
}

export async function addEntry(
  competitionId: string,
  personId: string,
): Promise<CompetitionEntry> {
  return invoke("add_entry", { competitionId, personId });
}

export async function reorderEntries(
  competitionId: string,
  entryIds: string[],
): Promise<CompetitionEntry[]> {
  return invoke("reorder_entries", { competitionId, entryIds });
}

export async function setEntryStatus(
  entryId: string,
  status: EntryStatus,
): Promise<CompetitionEntry> {
  return invoke("set_entry_status", { entryId, status });
}

/** @deprecated No-op; Nachkauf series counter increments on start. */
export async function setEntryNachkauf(
  entryId: string,
  nachkaufPurchased: number,
): Promise<CompetitionEntry> {
  return invoke("set_entry_nachkauf", { entryId, nachkaufPurchased });
}

export async function removeEntry(entryId: string): Promise<void> {
  await invoke("remove_entry", { entryId });
}

export async function cloneEntries(
  fromCompetitionId: string,
  toCompetitionId: string,
): Promise<CompetitionEntry[]> {
  return invoke("clone_entries", { fromCompetitionId, toCompetitionId });
}

export async function listCompetitionResults(
  competitionId: string,
): Promise<import("@rotpunktarena/domain").EntryResultSummary[]> {
  return invoke("list_competition_results", { competitionId });
}

export async function getEntryResult(
  entryId: string,
): Promise<import("@rotpunktarena/domain").EntryResultDetail | null> {
  return invoke("get_entry_result", { entryId });
}

export async function listEntrySeries(
  entryId: string,
): Promise<import("@rotpunktarena/domain").SeriesResultSummary[]> {
  return invoke("list_entry_series", { entryId });
}

export async function listTeams(
  competitionId?: string | null,
  includeArchived = false,
): Promise<CompetitionTeam[]> {
  return invoke("list_teams", {
    competitionId: competitionId ?? null,
    includeArchived,
  });
}

export async function listKnownTeamNames(
  includeArchived = false,
): Promise<string[]> {
  return invoke("list_known_team_names", { includeArchived });
}

export async function createTeam(
  nameOrCompetitionId: string,
  maybeName?: string,
): Promise<CompetitionTeam> {
  // Back-compat: createTeam(competitionId, name) or createTeam(name)
  const name = maybeName ?? nameOrCompetitionId;
  const competitionId = maybeName != null ? nameOrCompetitionId : null;
  return invoke("create_team", { name, competitionId });
}

export async function renameTeam(teamId: string, name: string): Promise<CompetitionTeam> {
  return invoke("rename_team", { teamId, name });
}

export async function setTeamArchived(
  teamId: string,
  archived: boolean,
): Promise<CompetitionTeam> {
  return invoke("set_team_archived", { teamId, archived });
}

export async function removeTeam(teamId: string): Promise<void> {
  await invoke("remove_team", { teamId });
}

export async function addTeamMember(
  teamId: string,
  entryId: string,
): Promise<CompetitionTeam> {
  return invoke("add_team_member", { teamId, entryId });
}

export async function removeTeamMember(
  teamId: string,
  entryId: string,
): Promise<CompetitionTeam> {
  return invoke("remove_team_member", { teamId, entryId });
}

export async function addTeamPerson(
  teamId: string,
  personId: string,
): Promise<CompetitionTeam> {
  return invoke("add_team_person", { teamId, personId });
}

export async function removeTeamPerson(
  teamId: string,
  personId: string,
): Promise<CompetitionTeam> {
  return invoke("remove_team_person", { teamId, personId });
}

export async function listTeamResults(
  competitionId: string,
): Promise<TeamResultSummary[]> {
  return invoke("list_team_results", { competitionId });
}
