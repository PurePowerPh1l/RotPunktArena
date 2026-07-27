import { useCallback, useEffect, useRef, useState } from "react";
import type {
  Competition,
  EntryResultDetail,
  EntryResultSummary,
  TeamResultSummary,
} from "@rotpunktarena/domain";
import * as api from "../../api/commands";
import { CompetitionPodium } from "../../components/CompetitionPodium";
import type { ScoreDisplayMode } from "../../components/TargetFace";
import { createRequestSeq } from "../../lib/requestSeq";
import { ResultsPanel } from "./ResultsPanel";
import { competitionListMeta } from "./labels";

export function CompetitionHistoryPanel() {
  const [competitions, setCompetitions] = useState<Competition[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [results, setResults] = useState<EntryResultSummary[]>([]);
  const [teamResults, setTeamResults] = useState<TeamResultSummary[]>([]);
  const [detail, setDetail] = useState<EntryResultDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [sortMode, setSortMode] = useState<ScoreDisplayMode>("punkte");
  const listSeq = useRef(createRequestSeq()).current;
  const resultsSeq = useRef(createRequestSeq()).current;

  const selected = competitions.find((c) => c.id === selectedId) ?? null;
  const teilerCompetition = selected?.scoringMode === "teiler";
  const teamsActive = Boolean(selected?.teamScoringEnabled);

  const reloadList = useCallback(async () => {
    const token = listSeq.begin();
    setLoading(true);
    setError(null);
    try {
      const list = await api.listCompetitions(true);
      if (!listSeq.isCurrent(token)) return;
      const past = list.filter(
        (c) => c.status === "closed" || c.status === "archived",
      );
      setCompetitions(past);
      setSelectedId((prev) => {
        if (prev && past.some((c) => c.id === prev)) return prev;
        return past[0]?.id ?? null;
      });
    } catch (e) {
      if (!listSeq.isCurrent(token)) return;
      setError(String(e));
      setCompetitions([]);
      setSelectedId(null);
    } finally {
      if (listSeq.isCurrent(token)) setLoading(false);
    }
  }, [listSeq]);

  const reloadResults = useCallback(async (competitionId: string) => {
    const token = resultsSeq.begin();
    setError(null);
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
      setError(String(e));
    }
  }, [resultsSeq]);

  useEffect(() => {
    void reloadList();
  }, [reloadList]);

  useEffect(() => {
    setDetail(null);
    if (!selectedId) {
      resultsSeq.begin();
      setResults([]);
      setTeamResults([]);
      return;
    }
    void reloadResults(selectedId);
  }, [selectedId, reloadResults, resultsSeq]);

  useEffect(() => {
    setSortMode(teilerCompetition ? "teiler" : "punkte");
  }, [selectedId, teilerCompetition]);

  const openResult = async (entryId: string) => {
    setError(null);
    try {
      const d = await api.getEntryResult(entryId);
      setDetail(d);
      if (!d) setError("Ergebnis nicht gefunden");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="hist-comp-layout">
      {error ? <p className="banner-error">{error}</p> : null}

      <div className="hist-comp-top">
        <section className="panel">
          <div className="trend-head">
            <h2>Vergangene Wettkämpfe</h2>
            <button
              type="button"
              className="secondary"
              onClick={() => void reloadList()}
            >
              Aktualisieren
            </button>
          </div>
          {loading ? (
            <p className="hint">Laden…</p>
          ) : competitions.length === 0 ? (
            <p className="hint">
              Noch keine geschlossenen oder archivierten Wettkämpfe.
            </p>
          ) : (
            <ul className="entity-list">
              {competitions.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    className={
                      c.id === selectedId ? "list-item active" : "list-item"
                    }
                    onClick={() => setSelectedId(c.id)}
                  >
                    <span className="list-title">{c.name}</span>
                    <span className="list-meta">{competitionListMeta(c)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        {selected ? (
          <CompetitionPodium
            competitionId={selected.id}
            results={results}
            teamResults={teamResults}
            sortMode={sortMode}
            onSortModeChange={setSortMode}
            teamsActive={teamsActive}
            onSelectEntry={(id) => void openResult(id)}
          />
        ) : (
          <div className="podium podium--hero podium--empty" aria-hidden>
            <p className="hint">Wettkampf wählen, um die Siegertreppe zu sehen.</p>
          </div>
        )}
      </div>

      {selected ? (
        <ResultsPanel
          selected={selected}
          results={results}
          teamResults={teamResults}
          detail={detail}
          teamsActive={teamsActive}
          teilerCompetition={teilerCompetition}
          sortMode={sortMode}
          onSortModeChange={setSortMode}
          onReload={() => void reloadResults(selected.id)}
          onOpenResult={(id) => void openResult(id)}
          onCloseDetail={() => setDetail(null)}
        />
      ) : null}
    </div>
  );
}
