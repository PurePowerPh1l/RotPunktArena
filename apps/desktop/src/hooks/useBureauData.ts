import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  Competition,
  CompetitionEntry,
  CompetitionTeam,
  Person,
} from "@reddot/domain";
import * as api from "../api/commands";
import { createRequestSeq } from "../lib/requestSeq";
import { useAsyncAction } from "./useAsyncAction";
import { createBureauMutations } from "./bureau/mutations";

type SelectedIdArg = string | null | ((prev: string | null) => string | null);

type Options = {
  /** Shared with Arena — when set, selection is controlled by the parent. */
  selectedId?: string | null;
  onSelectedIdChange?: (id: string | null) => void;
};

export function useBureauData(opts: Options = {}) {
  const controlled = opts.onSelectedIdChange != null;
  const [people, setPeople] = useState<Person[]>([]);
  const [competitions, setCompetitions] = useState<Competition[]>([]);
  const [internalSelectedId, setInternalSelectedId] = useState<string | null>(
    null,
  );
  const selectedId = controlled ? (opts.selectedId ?? null) : internalSelectedId;
  const onSelectedIdChange = opts.onSelectedIdChange;
  const selectedIdRef = useRef(selectedId);
  selectedIdRef.current = selectedId;

  const setSelectedId = useCallback(
    (arg: SelectedIdArg) => {
      const next =
        typeof arg === "function" ? arg(selectedIdRef.current) : arg;
      selectedIdRef.current = next;
      if (controlled) onSelectedIdChange?.(next);
      else setInternalSelectedId(next);
    },
    [controlled, onSelectedIdChange],
  );

  const [entries, setEntries] = useState<CompetitionEntry[]>([]);
  const [teams, setTeams] = useState<CompetitionTeam[]>([]);
  const [peopleQuery, setPeopleQuery] = useState("");
  const [includeArchived, setIncludeArchived] = useState(false);
  const [includeArchivedPeople, setIncludeArchivedPeople] = useState(false);
  const [includeArchivedTeams, setIncludeArchivedTeams] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const { busy, error: actionError, setError: setActionError, run } =
    useAsyncAction();

  const peopleSeq = useRef(createRequestSeq()).current;
  const competitionsSeq = useRef(createRequestSeq()).current;
  const entriesSeq = useRef(createRequestSeq()).current;
  const teamsSeq = useRef(createRequestSeq()).current;
  const refreshSeq = useRef(createRequestSeq()).current;

  const selected = competitions.find((c) => c.id === selectedId) ?? null;
  const error = actionError ?? loadError;

  const mutate = useCallback(
    async (fn: () => Promise<void>): Promise<boolean> => {
      setLoadError(null);
      const result = await run(fn);
      return result.ok;
    },
    [run],
  );

  const reloadPeople = useCallback(
    async (query?: string) => {
      const token = peopleSeq.begin();
      const list = await api.listPeople(query, includeArchivedPeople);
      if (!peopleSeq.isCurrent(token)) return;
      setPeople(list);
    },
    [includeArchivedPeople, peopleSeq],
  );

  const reloadCompetitions = useCallback(async () => {
    const token = competitionsSeq.begin();
    const list = await api.listCompetitions(includeArchived);
    if (!competitionsSeq.isCurrent(token)) return;
    setCompetitions(list);
    setSelectedId((prev) => {
      if (prev && list.some((c) => c.id === prev)) return prev;
      return list[0]?.id ?? null;
    });
  }, [includeArchived, competitionsSeq, setSelectedId]);

  const reloadEntries = useCallback(
    async (competitionId: string) => {
      const token = entriesSeq.begin();
      const list = await api.listEntries(competitionId);
      if (!entriesSeq.isCurrent(token)) return;
      setEntries(list);
    },
    [entriesSeq],
  );

  /** Global teams; pass competitionId to also fill memberEntryIds for start list. */
  const reloadTeams = useCallback(
    async (competitionId?: string | null) => {
      const token = teamsSeq.begin();
      const list = await api.listTeams(competitionId ?? null, includeArchivedTeams);
      if (!teamsSeq.isCurrent(token)) return;
      setTeams(list);
    },
    [includeArchivedTeams, teamsSeq],
  );

  const refreshAll = useCallback(async () => {
    const token = refreshSeq.begin();
    setLoading(true);
    setLoadError(null);
    try {
      await reloadCompetitions();
      await reloadPeople(peopleQuery);
      if (!refreshSeq.isCurrent(token)) return;
    } catch (e) {
      if (refreshSeq.isCurrent(token)) setLoadError(String(e));
    } finally {
      if (refreshSeq.isCurrent(token)) setLoading(false);
    }
  }, [peopleQuery, reloadCompetitions, reloadPeople, refreshSeq]);

  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  useEffect(() => {
    if (!selectedId) {
      entriesSeq.begin();
      setEntries([]);
      void reloadTeams(null).catch((e) => setLoadError(String(e)));
      return;
    }
    void reloadEntries(selectedId).catch((e) => setLoadError(String(e)));
    void reloadTeams(selectedId).catch((e) => setLoadError(String(e)));
  }, [selectedId, reloadEntries, reloadTeams, entriesSeq]);

  useEffect(() => {
    const t = window.setTimeout(() => {
      void reloadPeople(peopleQuery).catch((e) => setLoadError(String(e)));
    }, 180);
    return () => window.clearTimeout(t);
  }, [peopleQuery, reloadPeople]);

  const mutations = useMemo(
    () =>
      createBureauMutations({
        mutate,
        peopleQuery,
        selectedId,
        reloadPeople,
        reloadCompetitions,
        reloadEntries,
        reloadTeams,
        setSelectedId,
        setEntries,
      }),
    [
      mutate,
      peopleQuery,
      selectedId,
      reloadPeople,
      reloadCompetitions,
      reloadEntries,
      reloadTeams,
      setSelectedId,
    ],
  );

  const nextWaiting =
    entries.find((e) => e.status === "waiting" || e.status === "probe") ?? null;

  return {
    people,
    competitions,
    selected,
    selectedId,
    setSelectedId,
    entries,
    teams,
    peopleQuery,
    setPeopleQuery,
    includeArchived,
    setIncludeArchived,
    includeArchivedPeople,
    setIncludeArchivedPeople,
    includeArchivedTeams,
    setIncludeArchivedTeams,
    error,
    loading,
    busy,
    nextWaiting,
    ...mutations,
    refreshAll,
    setError: setActionError,
  };
}
