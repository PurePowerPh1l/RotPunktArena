import type { TrainingSessionSummary } from "@rotpunktarena/domain";
import { IconArchive, IconChevronRight, IconTraining } from "../../components/UiIcons";
import { formatScoreDe } from "../../lib/format";

type Props = {
  totalSessions: number;
  totalShots: number;
  archive: TrainingSessionSummary[];
  offset: number;
  busy: boolean;
  loading: boolean;
  selectedId: string | null;
  onLoad: () => void;
  onClose: () => void;
  onSelect: (id: string) => void;
};

export function TrainingInventoryPanel({ totalSessions, totalShots, archive, offset, busy, loading, selectedId, onLoad, onClose, onSelect }: Props) {
  const open = offset > 0;
  const hasMore = offset < totalSessions;
  const integer = (value: number) => value.toLocaleString("de-DE");
  return (
    <section className={`panel hist-inventory${open ? " is-open" : ""}`} aria-busy={busy || loading}>
      <div className="hist-inventory-overview">
        <div className="hist-inventory-title">
          <span className="hist-section-icon" aria-hidden><IconArchive size={22} /></span>
          <div><h2>Gesamter Trainingsbestand</h2><p>Alle Zeiträume · dauerhaft gespeichert</p></div>
        </div>
        <dl className="hist-inventory-metrics">
          <div><dt>Serien</dt><dd>{loading ? "—" : integer(totalSessions)}</dd></div>
          <div><dt>Schüsse</dt><dd>{loading ? "—" : integer(totalShots)}</dd></div>
        </dl>
        <button type="button" className="hist-archive-open" disabled={busy || loading || totalSessions === 0} onClick={open ? onClose : onLoad} aria-expanded={open} aria-controls="training-inventory-archive">
          {busy ? "Laden…" : open ? "Archiv schließen" : "Serienarchiv öffnen"}<IconChevronRight size={17} />
        </button>
      </div>
      {open ? <div id="training-inventory-archive" className="hist-inventory-archive">
        <div className="hist-archive-heading"><span><IconTraining size={15} /> Serienarchiv</span><span>{integer(offset - archive.length + 1)}–{integer(offset)} von {integer(totalSessions)}</span></div>
        <div className="hist-table-wrap hist-archive-table-wrap">
          <table className="hist-table hist-archive-table">
            <thead><tr><th>Datum</th><th>Schütze</th><th>Schüsse</th><th>Σ Punkte</th><th><span className="sr-only">Serie öffnen</span></th></tr></thead>
            <tbody>{[...archive].reverse().map((series) => <tr key={series.id} className={selectedId === series.id ? "hist-row-selected" : undefined}>
              <td><span className="hist-row-date">{new Date(series.endedAt).toLocaleDateString("de-DE")}</span><span className="hist-row-time">{new Date(series.endedAt).toLocaleTimeString("de-DE", { hour: "2-digit", minute: "2-digit" })}</span></td>
              <td>{series.shooterName}</td><td>{integer(series.shotCount)}</td><td className="hist-row-score">{formatScoreDe(series.punkteTotal)}</td>
              <td><button type="button" className="hist-row-open" onClick={() => onSelect(series.id)} aria-label={`Serie von ${series.shooterName} am ${new Date(series.endedAt).toLocaleString("de-DE")} öffnen`} title="Schussbild und Details öffnen"><IconChevronRight size={18} /></button></td>
            </tr>)}</tbody>
          </table>
        </div>
        <div className="hist-archive-footer"><span>{hasMore ? "Ältere Serien im Archiv verfügbar" : "Alle gespeicherten Serien angezeigt"}</span>{hasMore ? <button type="button" className="hist-small-action" disabled={busy || loading} onClick={onLoad}>{busy ? "Laden…" : "Ältere Serien laden"}<IconChevronRight size={15} /></button> : null}</div>
      </div> : null}
    </section>
  );
}
