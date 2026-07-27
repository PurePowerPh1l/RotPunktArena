import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type TransitionEvent,
} from "react";
import type {
  Competition,
  EntryResultDetail,
  EntryResultSummary,
  EntryStatus,
  TeamResultSummary,
} from "@rotpunktarena/domain";
import { EntryResultPanel } from "../../components/EntryResultPanel";
import { SearchSelect } from "../../components/SearchSelect";
import { SlidingSeg } from "../../components/SlidingSeg";
import type { ScoreDisplayMode } from "../../components/TargetFace";
import { IconPrint } from "../../components/UiIcons";
import { usePrintHotkey } from "../../hooks/usePrintHotkey";
import { formatPersonName, formatScoreCompact } from "../../lib/format";
import {
  compareByEntryRank,
  compareByTeamRank,
  entryRank,
  teamRank,
} from "../../lib/resultRank";
import {
  printEntryResultSheet,
  printResultsSheet,
} from "../../print/printSheets";
import { ENTRY_LABEL } from "./labels";

type Props = {
  selected: Competition;
  results: EntryResultSummary[];
  teamResults: TeamResultSummary[];
  detail: EntryResultDetail | null;
  teamsActive: boolean;
  /** Default sort when uncontrolled; also used for print sheet preference. */
  teilerCompetition: boolean;
  /** Controlled sort (Historie shares with podium). */
  sortMode?: ScoreDisplayMode;
  onSortModeChange?: (mode: ScoreDisplayMode) => void;
  onReload: () => void;
  onOpenResult: (entryId: string) => void;
  onCloseDetail: () => void;
};

/** all = no filter · none = entries without a team · otherwise teamId */
type TeamFilter = "all" | "none" | string;

type EntryTeamInfo = {
  teamId: string;
  teamName: string;
};

export function ResultsPanel({
  selected,
  results,
  teamResults,
  detail,
  teamsActive,
  teilerCompetition,
  sortMode: sortModeProp,
  onSortModeChange,
  onReload,
  onOpenResult,
  onCloseDetail,
}: Props) {
  const [teamFilter, setTeamFilter] = useState<TeamFilter>("all");
  const [internalSortMode, setInternalSortMode] = useState<ScoreDisplayMode>(
    teilerCompetition ? "teiler" : "punkte",
  );
  const [sheetDetail, setSheetDetail] = useState<EntryResultDetail | null>(null);
  const [sheetOpen, setSheetOpen] = useState(false);
  const closingRef = useRef(false);

  const controlled = sortModeProp !== undefined;
  const sortMode = controlled ? sortModeProp : internalSortMode;
  const setSortMode = onSortModeChange ?? setInternalSortMode;

  useEffect(() => {
    setTeamFilter("all");
    if (!controlled) {
      setInternalSortMode(teilerCompetition ? "teiler" : "punkte");
    }
  }, [selected.id, teilerCompetition, controlled]);

  useEffect(() => {
    if (detail) {
      closingRef.current = false;
      setSheetDetail(detail);
      const id = requestAnimationFrame(() => {
        requestAnimationFrame(() => setSheetOpen(true));
      });
      return () => cancelAnimationFrame(id);
    }
    setSheetOpen(false);
  }, [detail]);

  const requestCloseSheet = useCallback(() => {
    if (closingRef.current) return;
    closingRef.current = true;
    setSheetOpen(false);
    const reduce =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce) {
      setSheetDetail(null);
      closingRef.current = false;
      onCloseDetail();
    }
  }, [onCloseDetail]);

  const onSheetTransitionEnd = (e: TransitionEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    if (e.propertyName !== "opacity") return;
    if (sheetOpen) return;
    setSheetDetail(null);
    closingRef.current = false;
    onCloseDetail();
  };

  useEffect(() => {
    if (!sheetDetail) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") requestCloseSheet();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [sheetDetail, requestCloseSheet]);

  const entryTeamById = useMemo(() => {
    const map = new Map<string, EntryTeamInfo>();
    for (const team of teamResults) {
      for (const member of team.members) {
        map.set(member.entryId, {
          teamId: team.teamId,
          teamName: team.name,
        });
      }
    }
    return map;
  }, [teamResults]);

  const filteredResults = useMemo(() => {
    const base =
      !teamsActive || teamFilter === "all"
        ? results
        : results.filter((r) => {
            const team = entryTeamById.get(r.entryId);
            if (teamFilter === "none") return !team;
            return team?.teamId === teamFilter;
          });
    return [...base].sort((a, b) => compareByEntryRank(a, b, sortMode));
  }, [results, teamsActive, teamFilter, entryTeamById, sortMode]);

  const sortedTeams = useMemo(
    () =>
      [...teamResults].sort((a, b) => compareByTeamRank(a, b, sortMode)),
    [teamResults, sortMode],
  );

  const showVerein = results.some((r) => Boolean(r.club?.trim()));
  const colCount =
    6 + (showVerein ? 1 : 0) + (teamsActive ? 1 : 0) + 1; /* actions */

  const printCurrent = useCallback(() => {
    if (sheetDetail) {
      printEntryResultSheet({
        detail: sheetDetail,
        teilerMode: sortMode === "teiler",
      });
      return;
    }
    printResultsSheet({
      competition: selected,
      results: [...results].sort((a, b) => compareByEntryRank(a, b, sortMode)),
      teamResults: teamsActive
        ? [...teamResults].sort((a, b) => compareByTeamRank(a, b, sortMode))
        : undefined,
      teilerMode: sortMode === "teiler",
    });
  }, [
    sheetDetail,
    results,
    selected,
    teamResults,
    teamsActive,
    sortMode,
  ]);

  usePrintHotkey(printCurrent);

  return (
    <section className="panel results-panel">
      <div className="bureau-fill-head trend-head">
        <div>
          <h2>Ergebnisse</h2>
          <p className="panel-lead">
            {selected.nachkaufEnabled
              ? "Rangliste nach bester Serie — Zeile antippen für Schussbild."
              : "Zeile antippen für Serie und Schussbild."}
          </p>
        </div>
        <div className="trend-head-actions">
          <SlidingSeg
            size="sm"
            ariaLabel="Sortierung umschalten"
            value={sortMode}
            onChange={setSortMode}
            options={[
              { value: "punkte", label: "Punkte" },
              { value: "teiler", label: "Teiler" },
            ]}
          />
          {teamsActive && teamResults.length > 0 ? (
            <label className="results-team-filter">
              <SearchSelect
                value={teamFilter}
                options={[
                  { id: "all", label: "Alle Teams" },
                  ...teamResults.map((t) => ({
                    id: t.teamId,
                    label: t.name,
                  })),
                  { id: "none", label: "Ohne Team" },
                ]}
                onChange={(id) => setTeamFilter(id as TeamFilter)}
                placeholder="Team filtern…"
                allowClear={false}
              />
            </label>
          ) : null}
          <button type="button" className="secondary" onClick={printCurrent}>
            <IconPrint /> Drucken
          </button>
          <button type="button" className="secondary" onClick={onReload}>
            Aktualisieren
          </button>
        </div>
      </div>

      <div className="results-scroll">
        <div className="hist-table-wrap">
          <table className="hist-table">
            <thead>
              <tr>
                <th>Rang</th>
                <th>Name</th>
                {showVerein ? <th>Verein</th> : null}
                {teamsActive ? <th>Team</th> : null}
                <th>Status</th>
                <th>Schüsse</th>
                <th
                  className={
                    sortMode === "punkte" ? "hist-sort-active" : undefined
                  }
                >
                  Σ Punkte
                </th>
                <th
                  className={
                    sortMode === "teiler" ? "hist-sort-active" : undefined
                  }
                >
                  Ø Teiler
                </th>
                <th />
              </tr>
            </thead>
            <tbody>
              {filteredResults.map((r) => {
                const team = entryTeamById.get(r.entryId);
                const rank = entryRank(r, sortMode);
                return (
                  <tr key={r.entryId}>
                    <td>{rank ?? "—"}</td>
                    <td>{formatPersonName(r.lastName, r.firstName, "")}</td>
                    {showVerein ? (
                      <td className="results-meta-cell">
                        {r.club?.trim() || "—"}
                      </td>
                    ) : null}
                    {teamsActive ? (
                      <td className="results-meta-cell">
                        {team?.teamName ?? "—"}
                      </td>
                    ) : null}
                    <td>{ENTRY_LABEL[r.status as EntryStatus] ?? r.status}</td>
                    <td>
                      {r.shotCount}
                      {selected.maxShots > 0 ? ` / ${selected.maxShots}` : ""}
                    </td>
                    <td
                      className={
                        sortMode === "punkte" ? "hist-sort-active" : undefined
                      }
                    >
                      {r.shotCount === 0
                        ? "—"
                        : formatScoreCompact(r.punkteTotal)}
                    </td>
                    <td
                      className={
                        sortMode === "teiler" ? "hist-sort-active" : undefined
                      }
                    >
                      {r.shotCount === 0
                        ? "—"
                        : formatScoreCompact(r.teilerAvg)}
                    </td>
                    <td>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => onOpenResult(r.entryId)}
                      >
                        Anzeigen
                      </button>
                    </td>
                  </tr>
                );
              })}
              {filteredResults.length === 0 ? (
                <tr>
                  <td colSpan={colCount} className="empty">
                    {results.length === 0
                      ? "Keine Starter."
                      : "Keine Starter in diesem Team."}
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>

        {teamsActive && sortedTeams.length > 0 ? (
          <>
            <h3 className="subhead">Teams</h3>
            <div className="hist-table-wrap">
              <table className="hist-table">
                <thead>
                  <tr>
                    <th>Rang</th>
                    <th>Team</th>
                    <th
                      className={
                        sortMode === "punkte" ? "hist-sort-active" : undefined
                      }
                    >
                      Σ Punkte
                    </th>
                    <th
                      className={
                        sortMode === "teiler" ? "hist-sort-active" : undefined
                      }
                    >
                      Σ Teiler
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {sortedTeams.map((t) => (
                    <tr key={t.teamId}>
                      <td>{teamRank(t, sortMode) ?? "—"}</td>
                      <td>{t.name}</td>
                      <td
                        className={
                          sortMode === "punkte"
                            ? "hist-sort-active"
                            : undefined
                        }
                      >
                        {t.countingMembers === 0
                          ? "—"
                          : formatScoreCompact(t.punkteTotal)}
                      </td>
                      <td
                        className={
                          sortMode === "teiler"
                            ? "hist-sort-active"
                            : undefined
                        }
                      >
                        {t.countingMembers === 0
                          ? "—"
                          : formatScoreCompact(t.teilerSum)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        ) : null}
      </div>

      {sheetDetail ? (
        <div
          className={`result-sheet${sheetOpen ? " is-open" : ""}`}
          role="dialog"
          aria-modal="true"
          onTransitionEnd={onSheetTransitionEnd}
        >
          <button
            type="button"
            className="result-sheet-backdrop"
            aria-label="Schließen"
            onClick={requestCloseSheet}
          />
          <div className="result-sheet-panel">
            <EntryResultPanel
              detail={sheetDetail}
              onClose={requestCloseSheet}
              hotkeyOwnedExternally
            />
          </div>
        </div>
      ) : null}
    </section>
  );
}
