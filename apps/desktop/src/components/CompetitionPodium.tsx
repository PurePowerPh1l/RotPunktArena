import { useEffect, useMemo, useState, type CSSProperties } from "react";
import type {
  EntryResultSummary,
  TeamResultSummary,
} from "@reddot/domain";
import type { ScoreDisplayMode } from "./TargetFace";
import { formatPersonName, formatScoreCompact } from "../lib/format";
import {
  compareByEntryRank,
  compareByTeamRank,
  entryRank,
  teamRank,
} from "../lib/resultRank";
import { SlidingSeg } from "./SlidingSeg";
import { IconTrophy } from "./UiIcons";

export type PodiumPlace = {
  id: string;
  rank: 1 | 2 | 3;
  name: string;
  scoreLabel: string;
  secondaryLabel: string;
  entryId?: string;
};

export type PodiumMode = "einzel" | "teams";

type Props = {
  competitionId: string;
  results: EntryResultSummary[];
  teamResults: TeamResultSummary[];
  sortMode: ScoreDisplayMode;
  onSortModeChange: (mode: ScoreDisplayMode) => void;
  teamsActive: boolean;
  onSelectEntry?: (entryId: string) => void;
};

/** Top-3 from Rust ranks — display only, no score comparison. */
function podiumIndividuals(
  results: EntryResultSummary[],
  sortMode: ScoreDisplayMode,
): PodiumPlace[] {
  const teiler = sortMode === "teiler";
  return [...results]
    .filter((r) => entryRank(r, sortMode) != null)
    .sort((a, b) => compareByEntryRank(a, b, sortMode))
    .slice(0, 3)
    .map((r, i) => ({
      id: r.entryId,
      rank: (i + 1) as 1 | 2 | 3,
      name: formatPersonName(r.lastName, r.firstName, ""),
      scoreLabel: teiler
        ? formatScoreCompact(r.teilerAvg)
        : formatScoreCompact(r.punkteTotal),
      secondaryLabel: teiler
        ? `${formatScoreCompact(r.punkteTotal)} Pkt`
        : `Ø ${formatScoreCompact(r.teilerAvg)}`,
      entryId: r.entryId,
    }));
}

function podiumTeams(
  teamResults: TeamResultSummary[],
  sortMode: ScoreDisplayMode,
): PodiumPlace[] {
  const teiler = sortMode === "teiler";
  return [...teamResults]
    .filter((t) => teamRank(t, sortMode) != null)
    .sort((a, b) => compareByTeamRank(a, b, sortMode))
    .slice(0, 3)
    .map((t, i) => ({
      id: t.teamId,
      rank: (i + 1) as 1 | 2 | 3,
      name: t.name,
      scoreLabel: teiler
        ? formatScoreCompact(t.teilerSum)
        : formatScoreCompact(t.punkteTotal),
      secondaryLabel: teiler
        ? `${formatScoreCompact(t.punkteTotal)} Pkt`
        : `Σ ${formatScoreCompact(t.teilerSum)} T`,
    }));
}

const RANK_ORDER: Array<1 | 2 | 3> = [2, 1, 3];

export function CompetitionPodium({
  competitionId,
  results,
  teamResults,
  sortMode,
  onSortModeChange,
  teamsActive,
  onSelectEntry,
}: Props) {
  const individuals = useMemo(
    () => podiumIndividuals(results, sortMode),
    [results, sortMode],
  );
  const teams = useMemo(
    () => (teamsActive ? podiumTeams(teamResults, sortMode) : []),
    [teamResults, sortMode, teamsActive],
  );

  const canShowEinzel = individuals.length > 0;
  const canShowTeams = teams.length > 0;
  const showToggle = canShowEinzel && canShowTeams;

  const [mode, setMode] = useState<PodiumMode>("einzel");

  useEffect(() => {
    setMode(canShowEinzel ? "einzel" : "teams");
  }, [competitionId, canShowEinzel]);

  const activeMode: PodiumMode =
    mode === "teams" && canShowTeams
      ? "teams"
      : canShowEinzel
        ? "einzel"
        : "teams";

  const places = activeMode === "teams" ? teams : individuals;
  const byRank = new Map(places.map((p) => [p.rank, p]));
  const scoreUnit = sortMode === "teiler" ? "Teiler" : "Punkte";
  const empty = places.length === 0;

  return (
    <div className="podium podium--hero" key={competitionId}>
      <div className="podium-head">
        <span className="podium-kicker">
          <IconTrophy size={14} />
          {activeMode === "teams" ? "Team-Podium" : "Siegertreppe"}
        </span>
        <div className="podium-head-controls">
          {showToggle ? (
            <SlidingSeg
              size="sm"
              ariaLabel="Podium umschalten"
              className="podium-mode-seg"
              value={activeMode}
              onChange={setMode}
              options={[
                { value: "einzel", label: "Einzel" },
                { value: "teams", label: "Team" },
              ]}
            />
          ) : null}
          <SlidingSeg
            size="sm"
            ariaLabel="Sortierung umschalten"
            className="podium-sort-seg"
            value={sortMode}
            onChange={onSortModeChange}
            options={[
              { value: "punkte", label: "Punkte" },
              { value: "teiler", label: "Teiler" },
            ]}
          />
        </div>
      </div>

      {!empty ? (
        <span className="podium-sub podium-sub--below">
          Platz 1–{places.length} · sortiert nach {scoreUnit}
        </span>
      ) : null}

      {empty ? (
        <p className="hint podium-empty-hint">Noch keine Wertungen vorhanden.</p>
      ) : (
        <div
          className="podium-stage"
          role="list"
          aria-label={
            activeMode === "teams" ? "Team-Siegertreppe" : "Siegertreppe"
          }
          key={`${competitionId}-${activeMode}-${sortMode}`}
        >
          {RANK_ORDER.map((rank) => {
            const place = byRank.get(rank);
            if (!place) {
              return (
                <div key={rank} className="podium-slot podium-slot--empty" />
              );
            }

            const clickable = Boolean(place.entryId && onSelectEntry);
            const delayStyle = {
              "--podium-delay": `${rank === 1 ? 0.08 : rank === 2 ? 0.18 : 0.28}s`,
            } as CSSProperties;
            const slotClass = `podium-slot podium-slot--${rank}${clickable ? " podium-slot--clickable" : ""}${activeMode === "teams" ? " podium-slot--team" : ""}`;

            const body = (
              <>
                <div className="podium-athlete">
                  {rank === 1 ? (
                    <span className="podium-crown" aria-hidden>
                      <IconTrophy size={18} />
                    </span>
                  ) : null}
                  <span className="podium-name">{place.name}</span>
                  <span className="podium-score">{place.scoreLabel}</span>
                  <span className="podium-score-secondary">
                    {place.secondaryLabel}
                  </span>
                </div>
                <div className="podium-block">
                  <span className="podium-rank">{rank}</span>
                  <span className="podium-shine" aria-hidden />
                  {rank === 1 ? (
                    <span className="podium-glow" aria-hidden />
                  ) : null}
                </div>
              </>
            );

            if (clickable) {
              return (
                <button
                  key={place.id}
                  type="button"
                  role="listitem"
                  className={slotClass}
                  style={delayStyle}
                  onClick={() => onSelectEntry?.(place.entryId!)}
                >
                  {body}
                </button>
              );
            }

            return (
              <div
                key={place.id}
                role="listitem"
                className={slotClass}
                style={delayStyle}
              >
                {body}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
