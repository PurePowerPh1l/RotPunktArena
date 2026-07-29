import { useEffect, useRef } from "react";
import type { UiShot } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "./TargetFace";
import { IconTraining } from "./UiIcons";
import { shotRowValues } from "../hooks/useScoreDisplay";
import { formatScoreDe } from "../lib/format";

type Props = {
  shots: UiShot[];
  last?: UiShot | null;
  /** Best shot for current metric — only when series is complete. */
  best?: UiShot | null;
  /** User-selected shot (end-screen / results). */
  focusShot?: number | null;
  onFocusShot?: (shotIndex: number | null) => void;
  maxShots?: number | null;
  displayMode: ScoreDisplayMode;
};

/** Scrollable shot history — keeps the live score column from overflowing. */
export function ShotList({
  shots,
  last,
  best,
  focusShot = null,
  onFocusShot,
  maxShots,
  displayMode,
}: Props) {
  const end = useRef<HTMLDivElement>(null);
  const selectable = Boolean(onFocusShot);

  useEffect(() => {
    /* .score-col is the only scrollport — nearest keeps ceremony visible when possible */
    end.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [shots.length, last?.shotIndex]);

  if (shots.length === 0) {
    return (
      <div className="shot-list-wrap empty">
        <p className="shot-list-empty">
          <span className="shot-list-empty-ico" aria-hidden="true">
            <IconTraining size={22} />
          </span>
          Noch keine Schüsse
        </p>
      </div>
    );
  }

  const primaryLabel = displayMode === "teiler" ? "Teiler" : "Punkte";
  const secondaryLabel = displayMode === "teiler" ? "Punkte" : "Teiler";

  return (
    <div className="shot-list-wrap">
      <div className="shot-list-head">
        <span>
          Schussliste · {shots.length}
          {maxShots != null ? ` / ${maxShots}` : ""}
        </span>
        {shots.length > 12 ? <span className="shot-list-hint">scrollen</span> : null}
      </div>
      <div className="shot-list" tabIndex={0} aria-label="Frühere Schüsse">
        <table>
          <thead>
            <tr>
              <th>#</th>
              <th>{primaryLabel}</th>
              <th>{secondaryLabel}</th>
              <th>Σ</th>
            </tr>
          </thead>
          <tbody>
            {shots.map((s) => {
              const active = last?.shotIndex === s.shotIndex;
              const isBest = best != null && best.shotIndex === s.shotIndex;
              const focused = focusShot === s.shotIndex;
              const { primary, secondary, sigma } = shotRowValues(s, displayMode);
              const cls = [
                active ? "active" : "",
                isBest ? "best" : "",
                focused ? "focus" : "",
                selectable ? "shot-row-selectable" : "",
              ]
                .filter(Boolean)
                .join(" ");
              const toggleFocus = () =>
                onFocusShot?.(focusShot === s.shotIndex ? null : s.shotIndex);
              return (
                <tr
                  key={s.shotIndex}
                  className={cls || undefined}
                  onClick={selectable ? toggleFocus : undefined}
                  tabIndex={selectable ? 0 : undefined}
                  onKeyDown={
                    selectable
                      ? (e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            toggleFocus();
                          }
                        }
                      : undefined
                  }
                >
                  <td>
                    {s.shotIndex}
                    {isBest ? (
                      <span className="shot-best-tag" title="Bester Schuss">
                        {" "}
                        · Best
                      </span>
                    ) : null}
                    {focused ? (
                      <span className="shot-focus-tag" title="Ausgewählt">
                        {" "}
                        · Fokus
                      </span>
                    ) : null}
                  </td>
                  <td>{formatScoreDe(primary)}</td>
                  <td>{formatScoreDe(secondary)}</td>
                  <td>{formatScoreDe(sigma)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div ref={end} aria-hidden />
      </div>
    </div>
  );
}
