import type { TransferSnapshot } from "../../training/transfer";
import { formatTransferDelta } from "../../training/transfer";
import { fmtStat } from "../../training/stats";

type Props = {
  transfer: TransferSnapshot | null;
  loading?: boolean;
};

export function TrainingTransferPanel({ transfer, loading = false }: Props) {
  if (loading) {
    return (
      <section className="panel hist-transfer-panel" aria-label="Training vs Wettkampf">
        <h2>Training vs. Wettkampf</h2>
        <p className="hint">Lade Match-Ergebnisse…</p>
      </section>
    );
  }
  if (!transfer) return null;

  return (
    <section className="panel hist-transfer-panel" aria-label="Training vs Wettkampf">
      <div className="trend-head">
        <h2>Training vs. Wettkampf</h2>
      </div>
      <div className={`hist-transfer hist-transfer-${transfer.kind}`}>
        <div className="hist-transfer-cols">
          <div>
            <span className="hist-transfer-label">
              Training Ø ({transfer.trainingCount})
            </span>
            <strong>
              {transfer.trainingCount > 0
                ? fmtStat(transfer.trainingAvg)
                : "—"}
            </strong>
          </div>
          <div className="hist-transfer-delta" title="Training minus Match">
            {transfer.kind === "none"
              ? "—"
              : formatTransferDelta(transfer.delta)}
          </div>
          <div>
            <span className="hist-transfer-label">
              Match Ø ({transfer.competitionCount})
            </span>
            <strong>
              {transfer.competitionCount > 0
                ? fmtStat(transfer.competitionAvg)
                : "—"}
            </strong>
          </div>
        </div>
        <p className="hist-transfer-hint">{transfer.hint}</p>
      </div>
    </section>
  );
}
