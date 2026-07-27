import type { TrainingProgressSnapshot } from "../../training/seriesPulse";
import { fmtStat } from "../../training/stats";
import { LeagueBadge } from "../LeagueBadge";

type StripFocus = "xp" | "liga";

type Props = {
  /** Null = idle strip (no shooter yet) — keeps layout stable. */
  progress: TrainingProgressSnapshot | null;
  focus: StripFocus;
  onFocusChange: (focus: StripFocus) => void;
  rivalEnabled: boolean;
  onRivalEnabledChange: (on: boolean) => void;
  rivalTarget: { label: string; punkte: number } | null;
};

/** Compact XP / Liga strip for Arena training controls. */
export function TrainingProgressStrip({
  progress,
  focus,
  onFocusChange,
  rivalEnabled,
  onRivalEnabledChange,
  rivalTarget,
}: Props) {
  return (
    <div
      className={`train-arena-strip${progress ? "" : " is-idle"}`}
      aria-label="Training-Fortschritt"
    >
      <div
        key={progress ? "active" : "idle"}
        className="train-arena-strip-swap"
      >
        {progress ? (
          <ActiveStrip
            progress={progress}
            focus={focus}
            onFocusChange={onFocusChange}
            rivalEnabled={rivalEnabled}
            onRivalEnabledChange={onRivalEnabledChange}
            rivalTarget={rivalTarget}
          />
        ) : (
          <IdleStrip />
        )}
      </div>
    </div>
  );
}

function IdleStrip() {
  return (
    <>
      <div
        className="train-arena-strip-track is-empty"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={0}
        aria-label="XP-Fortschritt"
      />
      <p className="train-arena-strip-meta">
        Schütze wählen — dann XP & Liga
      </p>
    </>
  );
}

function ActiveStrip({
  progress,
  focus,
  onFocusChange,
  rivalEnabled,
  onRivalEnabledChange,
  rivalTarget,
}: {
  progress: TrainingProgressSnapshot;
  focus: StripFocus;
  onFocusChange: (focus: StripFocus) => void;
  rivalEnabled: boolean;
  onRivalEnabledChange: (on: boolean) => void;
  rivalTarget: { label: string; punkte: number } | null;
}) {
  const { stats, league } = progress;
  const xpPct = Math.round(stats.levelProgress * 100);
  const ligaPct = Math.round(league.progress * 100);

  const barPct = focus === "xp" ? xpPct : ligaPct;
  const barLabel =
    focus === "xp"
      ? stats.xpToNext > 0
        ? `noch ${fmtStat(stats.xpToNext, 0)} XP · Level ${stats.level + 1}`
        : "Max-Stufe"
      : league.tier === "unranked"
        ? `Platzierung · noch ${league.placementLeft} Serie${league.placementLeft === 1 ? "" : "n"}`
        : `${league.sr} SR · ${ligaPct}% zur nächsten Stufe`;

  return (
    <>
      <div className="train-arena-strip-head">
        <button
          type="button"
          className={`train-arena-strip-tab${focus === "xp" ? " is-active" : ""}`}
          onClick={() => onFocusChange("xp")}
          aria-pressed={focus === "xp"}
        >
          Level {stats.level}
          <span className="train-arena-strip-tab-sub">{stats.levelTitle}</span>
        </button>
        <button
          type="button"
          className={`train-arena-strip-tab${focus === "liga" ? " is-active" : ""}`}
          onClick={() => onFocusChange("liga")}
          aria-pressed={focus === "liga"}
        >
          <LeagueBadge rank={league} size="sm" showLabel />
        </button>
        <button
          type="button"
          className={`train-arena-strip-rival${rivalEnabled ? " is-active" : ""}`}
          onClick={() => onRivalEnabledChange(!rivalEnabled)}
          aria-pressed={rivalEnabled}
          title={
            rivalEnabled
              ? "Rival aus (Vergleich mit letzter eigener Serie)"
              : "Rival an — gegen deine letzte Serie"
          }
        >
          Rival
        </button>
      </div>
      <div
        className="train-arena-strip-track"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={barPct}
        aria-label={focus === "xp" ? "XP-Fortschritt" : "Liga-Fortschritt"}
      >
        <div
          className="train-arena-strip-fill"
          style={{ width: `${barPct}%` }}
        />
      </div>
      <p className="train-arena-strip-meta">
        {barLabel}
        {rivalEnabled && rivalTarget
          ? ` · Ziel ${fmtStat(rivalTarget.punkte)}`
          : null}
      </p>
    </>
  );
}
