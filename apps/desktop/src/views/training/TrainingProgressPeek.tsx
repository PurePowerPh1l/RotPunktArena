import type { LeagueRank } from "../../training/league";
import type { GoalProgress } from "../../training/goals";
import { formatGoalValue } from "../../training/goals";
import type { TrainingStats } from "../../training/stats";
import { fmtStat } from "../../training/stats";
import { LeagueBadge } from "../../components/LeagueBadge";
import { IconTrophy } from "../../components/UiIcons";

type Trend = { kind: string; text: string };

type Props = {
  filterLabel: string;
  singleShooter: boolean;
  stats: TrainingStats;
  league: LeagueRank | null;
  trend: Trend;
  topGoal?: GoalProgress | null;
};

/** Always-visible compact progress overview (no accordion required). */
export function TrainingProgressPeek({
  filterLabel,
  singleShooter,
  stats,
  league,
  trend,
  topGoal = null,
}: Props) {
  return (
    <div className="hist-progress-peek" aria-label="Fortschritt Überblick">
      <div className="hist-progress-peek-level">
        <div className="train-level-badge hist-peek-badge" aria-hidden>
          <IconTrophy size={18} />
          <span>{stats.level}</span>
        </div>
        <div className="hist-progress-peek-copy">
          <p className="hist-progress-peek-kicker">{filterLabel}</p>
          <p className="hist-progress-peek-title">
            {singleShooter ? stats.levelTitle : "Übersicht"}
            {singleShooter ? (
              <span className="train-level-sub"> Level {stats.level}</span>
            ) : null}
          </p>
          {singleShooter ? (
            <div
              className="train-xp-track hist-peek-xp"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(stats.levelProgress * 100)}
              aria-label="Fortschritt zum nächsten Level"
            >
              <div
                className="train-xp-fill"
                style={{ width: `${Math.round(stats.levelProgress * 100)}%` }}
              />
            </div>
          ) : (
            <p className="hint">Schütze wählen für Level & Liga.</p>
          )}
        </div>
        {singleShooter && league ? (
          <LeagueBadge rank={league} size="md" showLabel />
        ) : null}
      </div>

      <div className="hist-progress-peek-stats">
        <div className="hist-peek-stat">
          <span className="hist-peek-stat-label">Ø Serie</span>
          <strong>{fmtStat(stats.avgSeriePunkte)}</strong>
        </div>
        <div className="hist-peek-stat">
          <span className="hist-peek-stat-label">Beste</span>
          <strong>{fmtStat(stats.bestSerie)}</strong>
        </div>
        <div className={`hist-peek-stat peek-trend-${trend.kind}`}>
          <span className="hist-peek-stat-label">Trend</span>
          <strong>{trend.text}</strong>
        </div>
        {topGoal ? (
          <div className={`hist-peek-stat hist-peek-goal${topGoal.done ? " is-done" : ""}`}>
            <span className="hist-peek-stat-label">Ziel</span>
            <strong>
              {formatGoalValue(topGoal.goal.kind, topGoal.current)}
              <span className="hist-peek-goal-target">
                /{formatGoalValue(topGoal.goal.kind, topGoal.goal.target)}
              </span>
            </strong>
            <span
              className="hist-peek-goal-track"
              aria-hidden
            >
              <span
                className="hist-peek-goal-fill"
                style={{ width: `${Math.round(topGoal.ratio * 100)}%` }}
              />
            </span>
          </div>
        ) : null}
      </div>
    </div>
  );
}
