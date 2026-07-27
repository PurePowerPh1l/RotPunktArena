import { useEffect, useState } from "react";
import type { TrainingSessionDetail, UiShot } from "@rotpunktarena/domain";
import { ShotList } from "../../components/ShotList";
import { SlidingSeg } from "../../components/SlidingSeg";
import {
  TargetFace,
  type ScoreDisplayMode,
} from "../../components/TargetFace";
import { IconPrint } from "../../components/UiIcons";
import { bestShotOf } from "../../hooks/useScoreDisplay";
import { formatScoreCompact } from "../../lib/format";
import { printShotCard } from "../../print/printShotCard";
import { fmtStat } from "../../training/stats";

type Props = {
  detail: TrainingSessionDetail | null;
  onClose: () => void;
  loading?: boolean;
};

export function TrainingSeriesDetail({
  detail,
  onClose,
  loading = false,
}: Props) {
  const [displayMode, setDisplayMode] = useState<ScoreDisplayMode>("punkte");
  const [focusShot, setFocusShot] = useState<number | null>(null);

  useEffect(() => {
    setFocusShot(null);
  }, [detail?.summary.id]);

  if (!detail) {
    return (
      <section className="panel hist-detail-panel" aria-label="Serien-Detail">
        <div className="trend-head">
          <div>
            <p className="hist-series-detail-kicker">Serie im Fokus</p>
            <h3 className="hist-series-detail-title">Laden…</h3>
          </div>
          <button type="button" className="secondary" onClick={onClose}>
            Schließen
          </button>
        </div>
        <p className="hint hist-detail-loading">Schussbild wird geladen…</p>
      </section>
    );
  }

  const shots: UiShot[] = detail.shots;
  const last = shots[shots.length - 1] ?? null;
  const best = bestShotOf(shots, displayMode);

  const seriesPrimary =
    displayMode === "teiler"
      ? (last?.seriesTeilerTotal ?? detail.summary.teilerSum)
      : detail.summary.punkteTotal;

  return (
    <section className="panel hist-detail-panel" aria-label="Serien-Detail">
      <div className="trend-head">
        <div>
          <p className="hist-series-detail-kicker">Serie im Fokus</p>
          <h3 className="hist-series-detail-title">
            {detail.summary.shooterName}
          </h3>
          <p className="list-meta">
            {new Date(detail.summary.endedAt).toLocaleString("de-DE")}
            {" · "}
            {detail.summary.shotCount} Schüsse
            {" · "}
            {formatScoreCompact(seriesPrimary)}{" "}
            {displayMode === "teiler" ? "Σ Teiler" : "Σ Punkte"}
            {" · Ø Teiler "}
            {fmtStat(detail.summary.teilerAvg)}
          </p>
        </div>
        <div className="trend-head-actions">
          <SlidingSeg
            size="sm"
            ariaLabel="Kennzahl Detail"
            value={displayMode}
            onChange={setDisplayMode}
            options={[
              { value: "punkte", label: "Punkte" },
              { value: "teiler", label: "Teiler" },
            ]}
          />
          <button
            type="button"
            className="secondary"
            disabled={shots.length === 0 || loading}
            onClick={() =>
              printShotCard({
                shooterName: detail.summary.shooterName,
                modeLabel: "Training",
                shots,
                seriesTotal: detail.summary.punkteTotal,
                maxShots: detail.summary.shotCount,
                displayMode,
              })
            }
          >
            <IconPrint /> Drucken
          </button>
          <button type="button" className="secondary" onClick={onClose}>
            Schließen
          </button>
        </div>
      </div>

      <div
        key={detail.summary.id}
        className={`hist-series-detail-content${loading ? " is-loading" : ""}`}
      >
        {shots.length === 0 ? (
          <p className="hint">Keine Schussdaten für diese Serie.</p>
        ) : (
          <div className="hist-series-detail-body">
            <div className="hist-series-detail-face">
              <div className="face-wrap hist-face-wrap">
                <TargetFace
                  shots={shots}
                  last={last}
                  best={best}
                  focusShot={focusShot}
                  onFocusShot={setFocusShot}
                  interactive={false}
                  labelMode="value"
                  displayMode={displayMode}
                  allowInspect
                />
              </div>
            </div>
            <ShotList
              shots={shots}
              last={last}
              best={best}
              focusShot={focusShot}
              onFocusShot={setFocusShot}
              maxShots={detail.summary.shotCount}
              displayMode={displayMode}
            />
          </div>
        )}
      </div>
    </section>
  );
}
