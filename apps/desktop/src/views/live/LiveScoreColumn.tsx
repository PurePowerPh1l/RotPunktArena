import type { CompetitionEntry, UiShot } from "@rotpunktarena/domain";
import { useCallback, type ReactNode } from "react";
import { IconCheck, IconPerson, IconPrint } from "../../components/UiIcons";
import { ShotList } from "../../components/ShotList";
import { SlidingSeg } from "../../components/SlidingSeg";
import type { ScoreDisplayMode } from "../../components/TargetFace";
import { usePrintHotkey } from "../../hooks/usePrintHotkey";
import { formatPersonName, formatScoreDe } from "../../lib/format";
import { printShotCard } from "../../print/printShotCard";

type Props = {
  shooterFallback: string;
  sessionShooterName?: string | null;
  selectedEntry?: CompetitionEntry | null;
  displayMode: ScoreDisplayMode;
  onDisplayModeChange: (mode: ScoreDisplayMode) => void;
  scoreTick: number;
  primary: number | null;
  secondary: number | null;
  seriesPrimary: number;
  seriesTotalPunkte: number;
  last: UiShot | null;
  /** Best shot after series complete; null while shooting. */
  best?: UiShot | null;
  /** End-screen: selected shot on face + list. */
  focusShot?: number | null;
  onFocusShot?: (shotIndex: number | null) => void;
  shots: UiShot[];
  maxShots: number | null;
  shotCount: number;
  seriesComplete: boolean;
  detail: string | null;
  mode: "training" | "competition";
  nachkaufEnabled: boolean;
  nachkaufPurchased: number;
  endlessMode: boolean;
  onEndlessModeChange: (endless: boolean) => void;
  /** Probephase aktiv — Schüsse sind ungewertete Probeschüsse. */
  probeActive?: boolean;
  /** „Wertung beginnen“ — Probephase beenden. */
  onFinishProbe?: () => void;
  /** Training series-complete ceremony slot. */
  ceremony?: ReactNode;
  /** Optional rival target (training only) — shown as Ziel next to Serie. */
  rivalTarget?: { label: string; punkte: number } | null;
};

export function LiveScoreColumn({
  shooterFallback,
  sessionShooterName,
  selectedEntry,
  displayMode,
  onDisplayModeChange,
  scoreTick,
  primary,
  secondary,
  seriesPrimary,
  seriesTotalPunkte,
  last,
  best = null,
  focusShot = null,
  onFocusShot,
  shots,
  maxShots,
  shotCount,
  seriesComplete,
  detail,
  mode,
  nachkaufEnabled,
  nachkaufPurchased,
  endlessMode,
  onEndlessModeChange,
  probeActive = false,
  onFinishProbe,
  ceremony = null,
  rivalTarget = null,
}: Props) {
  const displayName =
    sessionShooterName ??
    (mode === "competition" && selectedEntry
      ? formatPersonName(selectedEntry.lastName, selectedEntry.firstName)
      : shooterFallback.trim() || "—");

  const doPrint = useCallback(() => {
    if (shotCount <= 0) return;
    printShotCard({
      shooterName: displayName,
      modeLabel: mode === "competition" ? "Wettkampf" : endlessMode ? "Training · Endlos" : "Training",
      shots,
      seriesTotal: displayMode === "teiler" ? seriesPrimary : seriesTotalPunkte,
      maxShots,
      displayMode,
    });
  }, [
    displayMode,
    displayName,
    endlessMode,
    maxShots,
    mode,
    seriesPrimary,
    seriesTotalPunkte,
    shotCount,
    shots,
  ]);

  usePrintHotkey(shotCount > 0 ? doPrint : null);

  const showEndlessToggle = mode === "training";
  const seriesNumber = 1 + Math.max(0, nachkaufPurchased);
  const entryDone =
    mode === "competition" && selectedEntry?.status === "done";
  const seriesHint =
    mode === "competition" && nachkaufEnabled && !probeActive
      ? seriesNumber > 1
        ? `Serie ${seriesNumber} (Nachkauf)`
        : "Serie 1"
      : null;

  return (
    <section className="score-col">
      <p className="shooter">
        <span className="shooter-label">
          <IconPerson size={12} /> Schütze
        </span>
        <span className="shooter-name-row">
          <span className="shooter-name">{displayName}</span>
          {entryDone ? (
            <span
              className="arena-fertig-chip"
              title={
                nachkaufEnabled
                  ? "Serie beendet — Nachkauf möglich"
                  : "Serie beendet"
              }
            >
              Fertig
            </span>
          ) : null}
        </span>
      </p>

      <div className="score-toolbar">
        <SlidingSeg
          className="score-seg"
          size="sm"
          ariaLabel="Anzeige"
          value={displayMode}
          onChange={onDisplayModeChange}
          options={[
            { value: "punkte", label: "Punkte" },
            { value: "teiler", label: "Teiler" },
          ]}
        />
        {shotCount > 0 ? (
          <button
            type="button"
            className="ghost score-print-btn nav-btn"
            onClick={doPrint}
            title="Schussbild drucken"
            aria-label="Schussbild drucken"
          >
            <IconPrint size={14} />
            Drucken
          </button>
        ) : null}
      </div>

      <p className="value value-tick" key={scoreTick} aria-live="polite">
        {primary != null ? formatScoreDe(primary) : "—"}
      </p>
      <p className="value-unit">
        {displayMode === "teiler" ? "Teiler" : "Punkte"}
      </p>
      <p className="meta">
        {probeActive ? "Probeschuss " : "Schuss "}
        {last?.shotIndex ?? 0}
        {probeActive
          ? " · ungewertet"
          : maxShots != null
            ? ` / ${maxShots}`
            : endlessMode
              ? " · Endlos"
              : null}
        {seriesHint ? ` · ${seriesHint}` : null}
        {secondary != null
          ? ` · ${displayMode === "teiler" ? "Punkte" : "Teiler"} ${formatScoreDe(secondary)}`
          : null}
      </p>
      <p className="total">
        {probeActive ? "Probe" : "Serie"}{" "}
        <strong key={`serie-${scoreTick}`} className="serie-tick">
          {formatScoreDe(seriesPrimary)}
        </strong>
        <span className="total-unit">
          {" "}
          {displayMode === "teiler" ? "Σ Teiler" : "Σ Punkte"}
        </span>
      </p>
      {mode === "training" && rivalTarget && displayMode === "punkte" ? (
        <p className="rival-target" title={`Rival: ${rivalTarget.label}`}>
          Ziel{" "}
          <strong>{formatScoreDe(rivalTarget.punkte)}</strong>
          <span className="rival-target-label"> · {rivalTarget.label}</span>
        </p>
      ) : null}
      {probeActive ? (
        <div className="probe-banner" aria-live="polite">
          <span className="probe-banner-label">Probe</span>
          <button
            type="button"
            className="probe-finish-btn"
            onClick={onFinishProbe}
            disabled={!onFinishProbe}
            title="Probeschüsse beenden — die gewertete Serie beginnt"
          >
            Wertung beginnen
          </button>
        </div>
      ) : null}
      <div className="series-done-slot" aria-live="polite">
        {ceremony}
        {!ceremony && seriesComplete ? (
          <p className="series-done">
            <IconCheck size={15} />
            Serie beendet
            {maxShots != null ? ` — ${shotCount}/${maxShots} Schüsse` : null}
          </p>
        ) : null}
      </div>
      {detail ? <p className="detail">{detail}</p> : null}

      <ShotList
        shots={shots}
        last={last}
        best={best}
        focusShot={focusShot}
        onFocusShot={onFocusShot}
        maxShots={maxShots}
        displayMode={displayMode}
      />

      {showEndlessToggle ? (
        <div className="score-actions">
          <label
            className="check-field endless-toggle"
            title="Unbegrenzt schießen — wird nicht in der Statistik gespeichert"
          >
            <input
              type="checkbox"
              checked={endlessMode}
              onChange={(e) => onEndlessModeChange(e.target.checked)}
            />
            Endlosmodus
          </label>
        </div>
      ) : null}
    </section>
  );
}
