import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UiShot } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "./TargetFace";
import { IconTraining } from "./UiIcons";
import { shotRowValues } from "../hooks/useScoreDisplay";
import { formatScoreDe } from "../lib/format";

type Props = {
  shots: UiShot[];
  sessionId?: string | null;
  probe?: boolean;
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
  sessionId,
  probe = false,
  last,
  best,
  focusShot = null,
  onFocusShot,
  maxShots,
  displayMode,
}: Props) {
  const [page, setPage] = useState<UiShot[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  useEffect(() => { request.current++; setPage(null); setBusy(false); setError(null); }, [sessionId, probe]);
  const visible = page ?? shots;
  const earlier = async () => {
    if (!sessionId || busy) return;
    const token = ++request.current;
    setBusy(true); setError(null);
    try {
      const result = await invoke<UiShot[]>("get_session_shot_page", { sessionId, probe, before: visible[0]?.shotIndex ?? null });
      if (request.current === token) setPage(result);
    } catch (e) { if (request.current === token) setError(String(e)); }
    finally { if (request.current === token) setBusy(false); }
  };
  const end = useRef<HTMLDivElement>(null);
  const selectable = Boolean(onFocusShot) && page == null;

  useEffect(() => {
    /* .score-col is the only scrollport — nearest keeps ceremony visible when possible */
    end.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [shots.length, last?.shotIndex]);

  if (visible.length === 0) {
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
          Schussliste · {last?.shotIndex ?? shots.length}
          {maxShots != null ? ` / ${maxShots}` : ""}
        </span>
        {(shots[0]?.shotIndex ?? 1) > 1 ? <span className="shot-list-hint">Letzte {shots.length} Treffer · ältere über Verlauf</span> : shots.length > 12 ? <span className="shot-list-hint">scrollen</span> : null}
      </div>
      {sessionId && ((visible[0]?.shotIndex ?? 1) > 1 || page) ? <div className="score-actions">
        <button className="btn ghost" disabled={busy || (visible[0]?.shotIndex ?? 1) <= 1} onClick={() => void earlier()}>�ltere 500 Sch�sse</button>
        {page ? <button className="btn ghost" onClick={() => { request.current++; setBusy(false); setPage(null); }}>Zur aktuellen Liste</button> : null}
        <span>{visible[0]?.shotIndex}�{visible[visible.length - 1]?.shotIndex}</span>
      </div> : null}
      {error ? <p role="alert">{error}</p> : null}
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
            {visible.map((s) => {
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
