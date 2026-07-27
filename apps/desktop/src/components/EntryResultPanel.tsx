import { useCallback, useEffect, useMemo, useState } from "react";
import type { EntryResultDetail, SeriesResultSummary, UiShot } from "@rotpunktarena/domain";
import { ShotList } from "./ShotList";
import { TargetFace, type ScoreDisplayMode } from "./TargetFace";
import { bestShotOf } from "../hooks/useScoreDisplay";
import { usePrintHotkey } from "../hooks/usePrintHotkey";
import { formatPersonName, formatScoreCompact } from "../lib/format";
import { printEntryResultSheet } from "../print/printSheets";

type Props = {
  detail: EntryResultDetail;
  onClose: () => void;
  /** When parent (ResultsPanel) already owns Ctrl+P. */
  hotkeyOwnedExternally?: boolean;
};

function seriesLabel(s: SeriesResultSummary): string {
  if (s.isNachkauf || s.seriesIndex > 1) {
    return `Serie ${s.seriesIndex} (Nachkauf)`;
  }
  return `Serie ${s.seriesIndex}`;
}

function seriesShots(s: SeriesResultSummary, fallbackBest: UiShot[]): UiShot[] {
  if (s.shots && s.shots.length > 0) return s.shots;
  if (s.isBest) return fallbackBest;
  return [];
}

export function EntryResultPanel({
  detail,
  onClose,
  hotkeyOwnedExternally = false,
}: Props) {
  const teilerMode = detail.scoringMode === "teiler";
  const [displayMode, setDisplayMode] = useState<ScoreDisplayMode>(
    teilerMode ? "teiler" : "punkte",
  );

  const seriesList = useMemo((): SeriesResultSummary[] => {
    if (detail.series && detail.series.length > 0) return detail.series;
    if (detail.shots.length === 0 && detail.summary.shotCount === 0) return [];
    return [
      {
        sessionId: detail.summary.sessionId ?? "best",
        seriesIndex: 1,
        endedAt: detail.summary.sessionEndedAt,
        shotCount: detail.summary.shotCount,
        punkteTotal: detail.summary.punkteTotal,
        teilerSum: detail.summary.teilerSum,
        teilerAvg: detail.summary.teilerAvg,
        isBest: true,
        isNachkauf: false,
        shots: detail.shots,
      },
    ];
  }, [detail]);

  const bestSessionId = useMemo(
    () => seriesList.find((s) => s.isBest)?.sessionId ?? seriesList[0]?.sessionId ?? null,
    [seriesList],
  );

  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
    bestSessionId,
  );
  const [focusShot, setFocusShot] = useState<number | null>(null);

  useEffect(() => {
    setSelectedSessionId(bestSessionId);
  }, [detail.summary.entryId, bestSessionId]);

  useEffect(() => {
    setFocusShot(null);
  }, [selectedSessionId, detail.summary.entryId]);

  const selected =
    seriesList.find((s) => s.sessionId === selectedSessionId) ??
    seriesList.find((s) => s.isBest) ??
    seriesList[0] ??
    null;

  const shots = selected ? seriesShots(selected, detail.shots) : [];
  const last = shots[shots.length - 1] ?? null;
  /** Results view = series complete → show best beside last. */
  const best = bestShotOf(shots, displayMode);
  const name = formatPersonName(detail.summary.lastName, detail.summary.firstName);

  const seriesPrimary = useMemo(() => {
    if (!selected) return 0;
    if (displayMode === "teiler") {
      return last?.seriesTeilerTotal ?? selected.teilerSum;
    }
    return selected.punkteTotal;
  }, [displayMode, last?.seriesTeilerTotal, selected]);

  const doPrint = useCallback(() => {
    printEntryResultSheet({
      detail,
      teilerMode: displayMode === "teiler",
    });
  }, [detail, displayMode]);

  usePrintHotkey(hotkeyOwnedExternally ? null : doPrint);

  const hasAnyShots =
    seriesList.some((s) => s.shotCount > 0) || detail.shots.length > 0;

  return (
    <div className="result-panel">
      <div className="trend-head">
        <div>
          <h3 className="result-title">{name}</h3>
          <p className="list-meta">
            {detail.competitionName}
            {detail.summary.sessionEndedAt
              ? ` · beste Serie ${new Date(detail.summary.sessionEndedAt).toLocaleString("de-DE")}`
              : " · keine Session"}
          </p>
        </div>
        <button type="button" className="ghost" onClick={onClose}>
          Schließen
        </button>
      </div>

      <div className="result-stats">
        <span>
          Beste Serie: {detail.summary.shotCount}
          {detail.maxShots > 0 ? ` / ${detail.maxShots}` : ""} Schüsse
        </span>
        <span>Σ Punkte {formatScoreCompact(detail.summary.punkteTotal)}</span>
        <span>Ø Teiler {formatScoreCompact(detail.summary.teilerAvg)}</span>
        {seriesList.length > 1 ? <span>{seriesList.length} Serien</span> : null}
      </div>

      {seriesList.length > 1 ? (
        <div className="hist-table-wrap result-series-overview">
          <table className="hist-table">
            <thead>
              <tr>
                <th>Serie</th>
                <th>Schüsse</th>
                <th>Σ Punkte</th>
                <th>Ø Teiler</th>
              </tr>
            </thead>
            <tbody>
              {seriesList.map((s) => {
                const active = s.sessionId === selected?.sessionId;
                return (
                  <tr
                    key={s.sessionId}
                    className={[
                      s.isBest ? "hist-best" : "",
                      active ? "result-series-row-active" : "",
                    ]
                      .filter(Boolean)
                      .join(" ") || undefined}
                    onClick={() => setSelectedSessionId(s.sessionId)}
                  >
                    <td>
                      {seriesLabel(s)}
                      {s.isBest ? " · Beste" : ""}
                    </td>
                    <td>
                      {s.shotCount}
                      {detail.maxShots > 0 ? ` / ${detail.maxShots}` : ""}
                    </td>
                    <td>
                      {s.shotCount === 0
                        ? "—"
                        : formatScoreCompact(s.punkteTotal)}
                    </td>
                    <td>
                      {s.shotCount === 0
                        ? "—"
                        : formatScoreCompact(s.teilerAvg)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      ) : null}

      <div className="seg" role="group" aria-label="Anzeige">
        <button
          type="button"
          className={displayMode === "punkte" ? "seg-on" : "secondary"}
          onClick={() => setDisplayMode("punkte")}
        >
          Punkte
        </button>
        <button
          type="button"
          className={displayMode === "teiler" ? "seg-on" : "secondary"}
          onClick={() => setDisplayMode("teiler")}
        >
          Teiler
        </button>
      </div>

      {!hasAnyShots ? (
        <p className="hint">Noch keine Schüsse für diesen Starter.</p>
      ) : !selected || shots.length === 0 ? (
        <p className="hint">Keine Schüsse in dieser Serie.</p>
      ) : (
        <div className="result-body">
          <div className="result-face">
            <div className="face-wrap">
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
          <div className="result-shots">
            <p className="total">
              {selected ? seriesLabel(selected) : "Serie"}{" "}
              <strong>{formatScoreCompact(seriesPrimary)}</strong>
              <span className="total-unit">
                {" "}
                {displayMode === "teiler" ? "Σ Teiler" : "Σ Punkte"}
              </span>
              {selected?.isBest ? (
                <span className="result-series-best-inline"> · Beste</span>
              ) : null}
            </p>
            <ShotList
              shots={shots}
              last={last}
              best={best}
              focusShot={focusShot}
              onFocusShot={setFocusShot}
              maxShots={detail.maxShots > 0 ? detail.maxShots : null}
              displayMode={displayMode}
            />
            <button
              type="button"
              className="secondary print-btn"
              onClick={doPrint}
            >
              Ergebnis drucken
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
