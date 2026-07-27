import type { TrainingSessionSummary } from "@reddot/domain";
import { IconPrint } from "../../components/UiIcons";
import { formatScoreCompact } from "../../lib/format";
import type { SeriesPulse } from "../../training/seriesPulse";
import { shareSummaryText } from "../../training/insights";
import { fmtStat } from "../../training/stats";

type Props = {
  sessions: TrainingSessionSummary[];
  newestFirst: TrainingSessionSummary[];
  loading: boolean;
  busy?: boolean;
  bestSerie: number;
  sessionCount: number;
  /** Soft pulse for the newest saved series (XP / Schnitt-Hinweis). */
  lastPulse?: SeriesPulse | null;
  selectedId: string | null;
  detailLoading?: boolean;
  detailError?: string | null;
  onSelect: (sessionId: string) => void;
  onPrint: () => void;
  onClear: () => void;
  onRefresh: () => void;
};

export function TrainingSeriesPanel({
  sessions,
  newestFirst,
  loading,
  busy = false,
  bestSerie,
  sessionCount,
  lastPulse = null,
  selectedId,
  detailLoading = false,
  detailError = null,
  onSelect,
  onPrint,
  onClear,
  onRefresh,
}: Props) {
  const latest = newestFirst[0] ?? null;

  const ligaText = (() => {
    if (!lastPulse) return null;
    if (lastPulse.leveledUp) {
      return `Level ${lastPulse.levelAfter} · ${lastPulse.levelTitleAfter}`;
    }
    if (lastPulse.srDelta != null) {
      const rounded = Math.round(lastPulse.srDelta);
      const sr =
        rounded > 0 ? `+${rounded} SR` : rounded < 0 ? `${rounded} SR` : "±0 SR";
      return `${sr} · ${lastPulse.league.label}`;
    }
    if (lastPulse.league.tier === "unranked") {
      const done = 5 - lastPulse.league.placementLeft;
      return `Platzierung ${done}/5`;
    }
    return lastPulse.league.label;
  })();

  const shareLatest = async () => {
    if (!latest) return;
    const text = shareSummaryText({
      shooterName: latest.shooterName,
      punkteTotal: latest.punkteTotal,
      shotCount: latest.shotCount,
      teilerAvg: latest.teilerAvg,
      xpGained: lastPulse?.xpGained,
      placeHint: lastPulse?.placeHint,
      endedAt: latest.endedAt,
    });
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      window.prompt("Zum Kopieren:", text);
    }
  };

  return (
    <section className="panel hist-series-panel">
      <div className="trend-head">
        <h2>Gespeicherte Serien</h2>
        <div className="trend-head-actions">
          <button
            type="button"
            className="secondary"
            disabled={busy || !latest}
            onClick={() => void shareLatest()}
            title="Kurz-Summary in die Zwischenablage"
          >
            Teilen
          </button>
          <button
            type="button"
            className="secondary"
            disabled={busy || sessions.length === 0}
            onClick={onPrint}
            title="Strg+P"
          >
            <IconPrint /> Drucken
          </button>
          <button
            type="button"
            className="secondary"
            disabled={busy || sessions.length === 0}
            onClick={onClear}
            title="Historie für den aktuellen Filter zurücksetzen"
          >
            Historie resetten
          </button>
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={onRefresh}
          >
            Aktualisieren
          </button>
        </div>
      </div>

      {latest && !loading ? (
        <article
          className={`hist-last-serie${lastPulse?.leveledUp ? " is-levelup" : ""}`}
          aria-label="Letzte Serie"
        >
          <button
            type="button"
            className="hist-last-serie-main hist-last-serie-btn"
            onClick={() => onSelect(latest.id)}
          >
            <p className="hist-last-serie-kicker">Letzte Serie</p>
            <p className="hist-last-serie-total">
              {formatScoreCompact(latest.punkteTotal)}
            </p>
            <p className="hist-last-serie-meta">
              {latest.shotCount} Schüsse · Ø Teiler {fmtStat(latest.teilerAvg)}
              {" · "}
              {new Date(latest.endedAt).toLocaleString("de-DE")}
            </p>
            {(lastPulse?.xpGained || ligaText || lastPulse?.placeHint) && (
              <p className="hist-last-serie-signals">
                {lastPulse && lastPulse.xpGained > 0 ? (
                  <span className="hist-last-serie-xp">
                    +{Math.round(lastPulse.xpGained)} XP
                  </span>
                ) : null}
                {ligaText ? (
                  <span
                    className={`hist-last-serie-liga${
                      lastPulse?.srDelta != null && lastPulse.srDelta < 0
                        ? " is-down"
                        : lastPulse?.srDelta != null && lastPulse.srDelta > 0
                          ? " is-up"
                          : ""
                    }`}
                  >
                    {ligaText}
                  </span>
                ) : null}
                {lastPulse?.placeHint ? (
                  <span
                    className={`hist-last-serie-hint hint-${lastPulse.placeHintKind}`}
                  >
                    {lastPulse.placeHint}
                  </span>
                ) : null}
              </p>
            )}
          </button>
          <div className="hist-last-serie-side">
            <span className="hist-last-serie-side-label">Schütze</span>
            <span className="hist-last-serie-side-value">{latest.shooterName}</span>
            {lastPulse != null ? (
              <span className="hist-last-serie-bar" aria-hidden>
                <span
                  className="hist-last-serie-bar-fill"
                  style={{
                    width: `${Math.round(lastPulse.levelProgressAfter * 100)}%`,
                  }}
                />
              </span>
            ) : null}
            <button
              type="button"
              className="secondary hist-last-serie-share"
              onClick={() => void shareLatest()}
            >
              Teilen
            </button>
          </div>
        </article>
      ) : null}

      {detailLoading ? (
        <p className="hint">Lade Serien-Detail…</p>
      ) : null}
      {detailError ? <p className="banner-error">{detailError}</p> : null}

      {newestFirst.length === 0 && !loading ? (
        <p className="hint">
          Noch keine Serien — beendete Trainings erscheinen automatisch.
        </p>
      ) : (
        <>
          <div className="hist-table-wrap hist-table-wrap-compact">
            <table className="hist-table">
              <thead>
                <tr>
                  <th>Datum</th>
                  <th>Schütze</th>
                  <th>Schüsse</th>
                  <th>Σ Punkte</th>
                  <th>Ø / Schuss</th>
                  <th>Ø Teiler</th>
                </tr>
              </thead>
              <tbody>
                {newestFirst.map((s, i) => {
                  const perShot = s.shotCount > 0 ? s.punkteTotal / s.shotCount : 0;
                  const isBest = s.punkteTotal === bestSerie && sessionCount > 0;
                  const isLatest = i === 0;
                  const isSelected = selectedId === s.id;
                  return (
                    <tr
                      key={s.id}
                      className={[
                        isBest ? "hist-best" : "",
                        isLatest ? "hist-latest" : "",
                        isSelected ? "hist-row-selected" : "",
                      ]
                        .filter(Boolean)
                        .join(" ") || undefined}
                      onClick={() => onSelect(s.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          onSelect(s.id);
                        }
                      }}
                      tabIndex={0}
                      role="button"
                      aria-pressed={isSelected}
                      title="Serie öffnen"
                    >
                      <td>{new Date(s.endedAt).toLocaleString("de-DE")}</td>
                      <td>{s.shooterName}</td>
                      <td>{s.shotCount}</td>
                      <td>{fmtStat(s.punkteTotal)}</td>
                      <td>{fmtStat(perShot)}</td>
                      <td>{fmtStat(s.teilerAvg)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          {newestFirst.length > 2 ? (
            <p className="hint hist-table-more">
              {newestFirst.length - 2} weitere Serie
              {newestFirst.length - 2 === 1 ? "" : "n"} — in der Liste scrollen
            </p>
          ) : null}
        </>
      )}
    </section>
  );
}
