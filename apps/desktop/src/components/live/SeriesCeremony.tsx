import { formatScoreCompact } from "../../lib/format";
import type { SeriesPulse } from "../../training/seriesPulse";

type Props = {
  open: boolean;
  seriesTotal: number;
  shotCount: number;
  maxShots: number | null;
  /** Soft pulse when series was saved into training history. */
  pulse?: SeriesPulse | null;
  /** Fallback XP estimate when history pulse is not ready yet. */
  xpPreview?: number | null;
  /** Open Statistik for the current shooter. */
  onOpenStats: () => void;
};

function fmtSrDelta(delta: number): string {
  const rounded = Math.round(delta);
  if (rounded > 0) return `+${rounded} SR`;
  if (rounded < 0) return `${rounded} SR`;
  return "±0 SR";
}

/** Compact training series-complete summary; stays until next series. */
export function SeriesCeremony({
  open,
  seriesTotal,
  shotCount,
  maxShots,
  pulse = null,
  xpPreview,
  onOpenStats,
}: Props) {
  if (!open) return null;

  const shotsLabel =
    maxShots != null ? `${shotCount} / ${maxShots} Schüsse` : `${shotCount} Schüsse`;

  const xpText =
    pulse != null
      ? `+${Math.round(pulse.xpGained)} XP`
      : xpPreview != null && xpPreview > 0
        ? `+${Math.round(xpPreview)} XP`
        : null;

  const ligaText = (() => {
    if (!pulse) return null;
    if (pulse.leveledUp) {
      return `Level ${pulse.levelAfter} · ${pulse.levelTitleAfter}`;
    }
    if (pulse.srDelta != null) {
      return `${fmtSrDelta(pulse.srDelta)} · ${pulse.league.label}`;
    }
    if (pulse.league.tier === "unranked") {
      const done = 5 - pulse.league.placementLeft;
      return `Platzierung ${done}/5`;
    }
    if (pulse.tierChanged) return pulse.league.label;
    return pulse.league.label;
  })();

  const progressPct = pulse
    ? Math.round(pulse.levelProgressAfter * 100)
    : null;

  return (
    <button
      type="button"
      className={`series-ceremony${pulse?.leveledUp ? " series-ceremony-levelup" : ""}`}
      onClick={onOpenStats}
      aria-label="Serie abgeschlossen — zur Statistik"
    >
      <span className="series-ceremony-kicker">
        {pulse?.leveledUp ? "Level Up" : "Serie beendet"}
      </span>
      <span className="series-ceremony-total">
        {formatScoreCompact(seriesTotal)}
      </span>
      <span className="series-ceremony-meta">{shotsLabel}</span>

      {(xpText || ligaText || pulse?.placeHint) && (
        <span className="series-ceremony-signals">
          {xpText ? (
            <span className="series-ceremony-xp">{xpText}</span>
          ) : null}
          {ligaText ? (
            <span
              className={`series-ceremony-liga${
                pulse?.srDelta != null && pulse.srDelta < 0
                  ? " is-down"
                  : pulse?.srDelta != null && pulse.srDelta > 0
                    ? " is-up"
                    : ""
              }`}
            >
              {ligaText}
            </span>
          ) : null}
          {pulse?.placeHint ? (
            <span
              className={`series-ceremony-hint hint-${pulse.placeHintKind}`}
            >
              {pulse.placeHint}
            </span>
          ) : null}
        </span>
      )}

      {progressPct != null ? (
        <span className="series-ceremony-bar" aria-hidden>
          <span
            className="series-ceremony-bar-fill"
            style={{ width: `${progressPct}%` }}
          />
        </span>
      ) : null}
    </button>
  );
}
