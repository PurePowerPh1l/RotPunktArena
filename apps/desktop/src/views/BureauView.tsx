import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  EntryResultDetail,
  EntryResultSummary,
  EntryStatus,
  TeamResultSummary,
} from "@rotpunktarena/domain";
import { requireAdminAuth } from "../access";
import { SlidingSeg } from "../components/SlidingSeg";
import { useBureauData } from "../hooks/useBureauData";
import { useEntryStartListDnD } from "../hooks/useEntryStartListDnD";
import { usePersonStartListDnD } from "../hooks/usePersonStartListDnD";
import { createRequestSeq } from "../lib/requestSeq";
import * as api from "../api/commands";
import { CompetitionChrome } from "./bureau/CompetitionChrome";
import { PeoplePanel } from "./bureau/PeoplePanel";
import { ResultsPanel } from "./bureau/ResultsPanel";
import {
  StartListPanel,
  type ArenaHandoff,
} from "./bureau/StartListPanel";
import { TeamsMasterPanel } from "./bureau/TeamsMasterPanel";

type Props = {
  onOpenLive?: (handoff?: ArenaHandoff) => void;
  adminMode?: boolean;
  /** Verwaltung selection (any status). */
  selectedCompetitionId?: string | null;
  onSelectedCompetitionIdChange?: (id: string | null) => void;
  /** Verwaltung selected/activated an active competition → Arena follows. */
  onActivateIntoArena?: (id: string) => void;
  /** Verwaltung demoted this competition → clear Arena if it was selected. */
  onClearArenaIfCompetition?: (id: string) => void;
  /** Create / status / update — Arena should reload its active list. */
  onCompetitionsInvalidated?: () => void;
};

type StammdatenTab = "people" | "teams";
type WettkampfTab = "startlist" | "results";

export function BureauView({
  onOpenLive,
  adminMode = false,
  selectedCompetitionId,
  onSelectedCompetitionIdChange,
  onActivateIntoArena,
  onClearArenaIfCompetition,
  onCompetitionsInvalidated,
}: Props) {
  const b = useBureauData({
    selectedId: selectedCompetitionId,
    onSelectedIdChange: onSelectedCompetitionIdChange,
  });

  const handleSelectCompetition = useCallback(
    (id: string | null) => {
      b.setSelectedId(id);
      if (!id) return;
      const c = b.competitions.find((x) => x.id === id);
      if (c?.status === "active") onActivateIntoArena?.(id);
    },
    [b.setSelectedId, b.competitions, onActivateIntoArena],
  );

  const handleCreateCompetition = useCallback(
    async (input: Parameters<typeof b.createCompetition>[0]) => {
      const id = await b.createCompetition(input);
      if (!id) return false;
      if (input.activateOnCreate) onActivateIntoArena?.(id);
      onCompetitionsInvalidated?.();
      return true;
    },
    [b.createCompetition, onActivateIntoArena, onCompetitionsInvalidated],
  );

  const handleUpdateCompetition = useCallback(
    async (id: string, input: Parameters<typeof b.updateCompetition>[1]) => {
      const ok = await b.updateCompetition(id, input);
      if (ok) onCompetitionsInvalidated?.();
      return ok;
    },
    [b.updateCompetition, onCompetitionsInvalidated],
  );

  const handleSetStatus = useCallback(
    async (status: Parameters<typeof b.setStatus>[0]) => {
      const id = b.selectedId;
      const ok = await b.setStatus(status);
      if (!ok || !id) return ok;
      if (status === "active") onActivateIntoArena?.(id);
      else onClearArenaIfCompetition?.(id);
      onCompetitionsInvalidated?.();
      return ok;
    },
    [
      b.selectedId,
      b.setStatus,
      onActivateIntoArena,
      onClearArenaIfCompetition,
      onCompetitionsInvalidated,
    ],
  );

  const handleCreateFromTemplate = useCallback(
    async (sourceId: string) => {
      const ok = await b.createFromTemplate(sourceId);
      if (ok) onCompetitionsInvalidated?.();
      return ok;
    },
    [b.createFromTemplate, onCompetitionsInvalidated],
  );

  const handleSaveAsTemplate = useCallback(async () => {
    const ok = await b.saveAsTemplate();
    if (ok) onCompetitionsInvalidated?.();
    return ok;
  }, [b.saveAsTemplate, onCompetitionsInvalidated]);

  const [stammdatenTab, setStammdatenTab] = useState<StammdatenTab>("people");
  const [wettkampfTab, setWettkampfTab] = useState<WettkampfTab>("startlist");
  const [results, setResults] = useState<EntryResultSummary[]>([]);
  const [teamResults, setTeamResults] = useState<TeamResultSummary[]>([]);
  const [detail, setDetail] = useState<EntryResultDetail | null>(null);
  const [resultsError, setResultsError] = useState<string | null>(null);
  const [listDropActive, setListDropActive] = useState(false);

  const startListLocked = b.selected?.status === "closed";
  const startListEditEnabled =
    Boolean(b.selectedId) &&
    !startListLocked &&
    !b.busy &&
    wettkampfTab === "startlist";

  const personDnD = usePersonStartListDnD({
    enabled: startListEditEnabled,
    onDropPerson: (personId) => b.addPersonToStartList(personId),
  });

  const entryDnD = useEntryStartListDnD({
    enabled: startListEditEnabled,
    onReorder: async (draggedId, targetId) => {
      const ids = b.entries.map((e) => e.id);
      const from = ids.indexOf(draggedId);
      const to = ids.indexOf(targetId);
      if (from < 0 || to < 0 || from === to) return;
      ids.splice(from, 1);
      ids.splice(to, 0, draggedId);
      await b.reorderEntries(ids);
    },
    onRemove: async (entryId) => {
      const entry = b.entries.find((e) => e.id === entryId);
      if (!entry) return;
      if (entry.status === "done") {
        if (!(await requireAdminAuth())) return;
      }
      await b.removeEntry(entryId);
    },
    canRemove: (entryId) => Boolean(b.entries.find((e) => e.id === entryId)),
  });

  const requireAdminForDoneEntry = useCallback(
    async (entryId: string): Promise<boolean> => {
      const entry = b.entries.find((e) => e.id === entryId);
      if (!entry || entry.status !== "done") return true;
      return requireAdminAuth();
    },
    [b.entries],
  );

  const setEntryStatusGuarded = useCallback(
    async (entryId: string, status: EntryStatus) => {
      if (!(await requireAdminForDoneEntry(entryId))) return false;
      return b.setEntryStatus(entryId, status);
    },
    [b, requireAdminForDoneEntry],
  );

  const removeEntryGuarded = useCallback(
    async (entryId: string) => {
      if (!(await requireAdminForDoneEntry(entryId))) return false;
      return b.removeEntry(entryId);
    },
    [b, requireAdminForDoneEntry],
  );

  const inList = useMemo(
    () => new Set(b.entries.map((e) => e.personId)),
    [b.entries],
  );

  const doneInList = useMemo(
    () =>
      new Set(
        b.entries.filter((e) => e.status === "done").map((e) => e.personId),
      ),
    [b.entries],
  );

  const teamByEntry = useMemo(() => {
    const m = new Map<string, string>();
    const personToTeam = new Map<string, string>();
    for (const t of b.teams) {
      if (t.archived) continue;
      for (const pid of t.memberPersonIds ?? []) personToTeam.set(pid, t.id);
      for (const eid of t.memberEntryIds) m.set(eid, t.id);
    }
    for (const e of b.entries) {
      const tid = personToTeam.get(e.personId);
      if (tid) m.set(e.id, tid);
    }
    return m;
  }, [b.teams, b.entries]);

  const teilerCompetition = b.selected?.scoringMode === "teiler";
  const teamsActive = Boolean(b.selected?.teamScoringEnabled);

  /** Reload results only when status/membership changes — not on reorder. */
  const resultsEpoch = useMemo(
    () =>
      [
        ...b.entries.map((e) => `${e.id}:${e.status}`),
        ...b.teams.map((t) => {
          const people = [...(t.memberPersonIds ?? [])].sort().join(",");
          const entries = [...t.memberEntryIds].sort().join(",");
          return `${t.id}:${people}:${entries}`;
        }),
      ].join("\0"),
    [b.entries, b.teams],
  );

  const resultsSeq = useRef(createRequestSeq()).current;

  const reloadResults = useCallback(async (competitionId: string) => {
    const token = resultsSeq.begin();
    setResultsError(null);
    try {
      const [nextResults, nextTeams] = await Promise.all([
        api.listCompetitionResults(competitionId),
        api.listTeamResults(competitionId),
      ]);
      if (!resultsSeq.isCurrent(token)) return;
      setResults(nextResults);
      setTeamResults(nextTeams);
    } catch (e) {
      if (!resultsSeq.isCurrent(token)) return;
      setResults([]);
      setTeamResults([]);
      setResultsError(String(e));
    }
  }, [resultsSeq]);

  useEffect(() => {
    setDetail(null);
    setWettkampfTab("startlist");
    if (!b.selectedId) {
      resultsSeq.begin();
      setResults([]);
      setTeamResults([]);
    }
  }, [b.selectedId, resultsSeq]);

  useEffect(() => {
    if (!b.selectedId) return;
    void reloadResults(b.selectedId);
  }, [b.selectedId, resultsEpoch, reloadResults]);

  const openResult = async (entryId: string) => {
    setResultsError(null);
    try {
      const d = await api.getEntryResult(entryId);
      setDetail(d);
      if (!d) setResultsError("Ergebnis nicht gefunden");
    } catch (e) {
      setResultsError(String(e));
    }
  };

  const setEntryTeam = async (entryId: string, teamId: string) => {
    if (!teamId) {
      const current = teamByEntry.get(entryId);
      if (current) return b.removeTeamMember(current, entryId);
      return true;
    }
    return b.addTeamMember(teamId, entryId);
  };

  const dragGhost = personDnD.ghost ?? entryDnD.ghost;

  return (
    <div
      className={`bureau${personDnD.dragging ? " is-person-dragging" : ""}${entryDnD.dragging ? " is-entry-dragging" : ""}`}
    >
      <div className="bureau-main">
        {b.error ? <p className="banner-error">{b.error}</p> : null}
        {resultsError ? <p className="banner-error">{resultsError}</p> : null}

        <div className="bureau-grid">
          <div className="bureau-zone bureau-zone-stammdaten">
            <div className="bureau-zone-head">
              <SlidingSeg
                className="bureau-stammdaten-seg"
                size="sm"
                ariaLabel="Stammdaten"
                value={stammdatenTab}
                onChange={setStammdatenTab}
                options={[
                  { value: "people", label: "Schützen" },
                  { value: "teams", label: "Teams" },
                ]}
              />
            </div>
            <div className="bureau-zone-body">
              {stammdatenTab === "people" ? (
                <PeoplePanel
                  people={b.people}
                  peopleQuery={b.peopleQuery}
                  selectedId={b.selectedId}
                  inList={inList}
                  busy={b.busy}
                  includeArchived={b.includeArchivedPeople}
                  onIncludeArchivedChange={b.setIncludeArchivedPeople}
                  onPeopleQueryChange={b.setPeopleQuery}
                  onCreate={b.createPerson}
                  onUpdate={b.updatePerson}
                  onDelete={b.deletePerson}
                  onSetArchived={b.setPersonArchived}
                  onAddToStartList={b.addPersonToStartList}
                  onRemoveFromStartList={async (personId) => {
                    const entry = b.entries.find((e) => e.personId === personId);
                    if (!entry) return false;
                    if (entry.status === "done") {
                      if (!(await requireAdminAuth())) return false;
                    }
                    return b.removeEntry(entry.id);
                  }}
                  doneInList={doneInList}
                  nachkaufEnabled={Boolean(b.selected?.nachkaufEnabled)}
                  startListLocked={startListLocked}
                  onBeginPersonDrag={personDnD.beginPersonDrag}
                />
              ) : (
                <TeamsMasterPanel
                  teams={b.teams}
                  people={b.people}
                  busy={b.busy}
                  includeArchived={b.includeArchivedTeams}
                  onIncludeArchivedChange={b.setIncludeArchivedTeams}
                  onCreate={b.createTeam}
                  onRename={b.renameTeam}
                  onSetArchived={b.setTeamArchived}
                  onDelete={b.removeTeam}
                  onAddPerson={b.addTeamPerson}
                  onRemovePerson={b.removeTeamPerson}
                />
              )}
            </div>
          </div>

          <div className="bureau-zone bureau-zone-wettkampf">
            <CompetitionChrome
              competitions={b.competitions}
              selected={b.selected}
              selectedId={b.selectedId}
              busy={b.busy}
              includeArchived={b.includeArchived}
              onIncludeArchivedChange={b.setIncludeArchived}
              onSelect={handleSelectCompetition}
              onCreate={handleCreateCompetition}
              onUpdate={handleUpdateCompetition}
              onSetStatus={handleSetStatus}
              onSaveAsTemplate={handleSaveAsTemplate}
              onCreateFromTemplate={handleCreateFromTemplate}
            />

            {b.selected && b.selectedId ? (
              <div className="bureau-wettkampf-body">
                <div className="bureau-zone-head">
                  <SlidingSeg
                    className="bureau-wettkampf-seg"
                    size="sm"
                    ariaLabel="Wettkampf-Ansicht"
                    value={wettkampfTab}
                    onChange={setWettkampfTab}
                    options={[
                      { value: "startlist", label: "Startliste" },
                      { value: "results", label: "Ergebnisse" },
                    ]}
                  />
                </div>
                <div className="bureau-wettkampf-slot">
                  {wettkampfTab === "startlist" ? (
                    <StartListPanel
                      selected={b.selected}
                      selectedId={b.selectedId}
                      competitions={b.competitions}
                      entries={b.entries}
                      teams={b.teams}
                      teamsActive={teamsActive}
                      busy={b.busy}
                      adminMode={adminMode}
                      teamByEntry={teamByEntry}
                      dragOverEntry={entryDnD.overEntryId}
                      draggingEntryId={entryDnD.ghost?.entryId ?? null}
                      listDropActive={listDropActive || personDnD.overDrop}
                      onListDropActive={setListDropActive}
                      onAddByName={b.addShooterByName}
                      onSetEntryStatus={setEntryStatusGuarded}
                      onRemoveEntry={removeEntryGuarded}
                      onArchivePerson={(personId) =>
                        b.setPersonArchived(personId, true)
                      }
                      onSetEntryTeam={setEntryTeam}
                      onCloneFrom={b.cloneFrom}
                      onOpenLive={(handoff) => onOpenLive?.(handoff)}
                      onBeginEntryDrag={entryDnD.beginEntryDrag}
                      onOpenEntryResults={(entryId) => {
                        setWettkampfTab("results");
                        void openResult(entryId);
                      }}
                    />
                  ) : (
                    <ResultsPanel
                      selected={b.selected}
                      results={results}
                      teamResults={teamResults}
                      detail={detail}
                      teamsActive={teamsActive}
                      teilerCompetition={teilerCompetition}
                      onReload={() => void reloadResults(b.selectedId!)}
                      onOpenResult={(id) => void openResult(id)}
                      onCloseDetail={() => setDetail(null)}
                    />
                  )}
                </div>
              </div>
            ) : (
              <div className="bureau-wettkampf-empty panel">
                <p className="bureau-empty-title">Wettkampf wählen oder anlegen</p>
                <p className="empty-soft">
                  Oben einen bestehenden Wettkampf öffnen — oder mit „Neu“ starten.
                  Danach erscheinen Startliste und Ergebnisse hier.
                </p>
              </div>
            )}
          </div>
        </div>
      </div>

      {dragGhost ? (
        <div
          className={`person-drag-ghost${personDnD.overDrop ? " is-over-drop" : ""}${entryDnD.removing ? " is-removing" : ""}${entryDnD.overEntryId ? " is-over-drop" : ""}`}
          style={{
            left: dragGhost.x + 12,
            top: dragGhost.y + 12,
          }}
        >
          {entryDnD.removing ? `Entfernen · ${dragGhost.label}` : dragGhost.label}
        </div>
      ) : null}
    </div>
  );
}
