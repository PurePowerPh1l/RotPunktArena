import { useMemo, useState } from "react";
import type { TrainingSessionSummary } from "@reddot/domain";
import { SlidingSeg } from "../../components/SlidingSeg";
import {
  createGoal,
  evaluateGoals,
  formatGoalValue,
  GOAL_KIND_OPTIONS,
  type GoalKind,
  type TrainingGoal,
} from "../../training/goals";
import {
  HISTORY_WINDOW_OPTIONS,
  type HistoryWindowDays,
} from "../../training/insights";

type Props = {
  sessions: TrainingSessionSummary[];
  goals: TrainingGoal[];
  onChange: (next: TrainingGoal[]) => void;
  disabled?: boolean;
};

export function TrainingGoalsPanel({
  sessions,
  goals,
  onChange,
  disabled = false,
}: Props) {
  const [kind, setKind] = useState<GoalKind>("avgSerie");
  const [target, setTarget] = useState("95");
  const [windowDays, setWindowDays] = useState<HistoryWindowDays>(30);
  const progress = useMemo(
    () => evaluateGoals(sessions, goals),
    [sessions, goals],
  );

  const add = () => {
    const n = Number(target.replace(",", "."));
    if (!Number.isFinite(n) || n <= 0) return;
    if (goals.length >= 4) return;
    onChange([...goals, createGoal(kind, n, windowDays)]);
  };

  const remove = (id: string) => {
    onChange(goals.filter((g) => g.id !== id));
  };

  return (
    <section className="panel hist-goals-panel" aria-label="Ziele">
      <div className="trend-head">
        <h2>Ziele</h2>
      </div>

      {progress.length === 0 ? (
        <p className="hint">
          Setze ein Ziel (z. B. Ø 95 diese Woche) — Fortschritt erscheint hier.
        </p>
      ) : (
        <ul className="hist-goal-list">
          {progress.map((p) => (
            <li
              key={p.goal.id}
              className={p.done ? "hist-goal is-done" : "hist-goal"}
            >
              <div className="hist-goal-head">
                <span className="hist-goal-label">{p.label}</span>
                <button
                  type="button"
                  className="hist-goal-remove"
                  disabled={disabled}
                  onClick={() => remove(p.goal.id)}
                  aria-label="Ziel entfernen"
                >
                  ×
                </button>
              </div>
              <p className="hist-goal-values">
                {formatGoalValue(p.goal.kind, p.current)}
                <span> / {formatGoalValue(p.goal.kind, p.goal.target)}</span>
                {p.done ? <span className="hist-goal-done"> erreicht</span> : null}
              </p>
              <div
                className="hist-goal-track"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(p.ratio * 100)}
              >
                <div
                  className="hist-goal-fill"
                  style={{ width: `${Math.round(p.ratio * 100)}%` }}
                />
              </div>
            </li>
          ))}
        </ul>
      )}

      {goals.length < 4 ? (
        <div className="hist-goal-form">
          <label className="hist-goal-field">
            <span>Art</span>
            <select
              value={kind}
              disabled={disabled}
              onChange={(e) => setKind(e.target.value as GoalKind)}
            >
              {GOAL_KIND_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>
          <label className="hist-goal-field">
            <span>Zielwert</span>
            <input
              type="text"
              inputMode="decimal"
              value={target}
              disabled={disabled}
              onChange={(e) => setTarget(e.target.value)}
            />
          </label>
          <div className="hist-goal-field">
            <span>Fenster</span>
            <SlidingSeg
              size="sm"
              ariaLabel="Ziel-Zeitfenster"
              value={windowDays === null ? "all" : String(windowDays)}
              onChange={(v) =>
                setWindowDays(v === "all" ? null : (Number(v) as HistoryWindowDays))
              }
              options={HISTORY_WINDOW_OPTIONS.map((o) => ({
                value: o.value === null ? "all" : String(o.value),
                label: o.label,
              }))}
            />
          </div>
          <button
            type="button"
            className="secondary"
            disabled={disabled}
            onClick={add}
          >
            Ziel setzen
          </button>
        </div>
      ) : null}
    </section>
  );
}
