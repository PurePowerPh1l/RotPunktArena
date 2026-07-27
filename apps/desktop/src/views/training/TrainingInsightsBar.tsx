import type { CompareBanner, FormInsight } from "../../training/insights";
import { fmtStat } from "../../training/stats";

type Props = {
  compare: CompareBanner | null;
  insights: FormInsight[];
};

export function TrainingInsightsBar({ compare, insights }: Props) {
  if (!compare && insights.length === 0) return null;

  return (
    <section className="panel hist-insights" aria-label="Form & Vergleich">
      {compare ? (
        <div className={`hist-compare hist-compare-${compare.kind}`}>
          <div className="hist-compare-main">
            <p className="hist-compare-kicker">Letzte Serie vs. Schnitt</p>
            <p className="hist-compare-total">
              {fmtStat(compare.lastPunkte)}
              {compare.deltaVs5 != null ? (
                <span className="hist-compare-delta">
                  {compare.deltaVs5 > 0 ? "+" : ""}
                  {fmtStat(compare.deltaVs5)}
                </span>
              ) : null}
            </p>
            <p className="hist-compare-hint">{compare.hint}</p>
          </div>
          <div className="hist-compare-grid">
            <div>
              <span className="hist-compare-label">Ø 5</span>
              <strong>
                {compare.avg5 != null ? fmtStat(compare.avg5) : "—"}
              </strong>
            </div>
            <div>
              <span className="hist-compare-label">Ø 10</span>
              <strong>
                {compare.avg10 != null ? fmtStat(compare.avg10) : "—"}
              </strong>
            </div>
            <div>
              <span className="hist-compare-label">Beste Woche</span>
              <strong>
                {compare.bestWeek != null ? fmtStat(compare.bestWeek) : "—"}
              </strong>
            </div>
          </div>
        </div>
      ) : null}

      {insights.length > 0 ? (
        <div className="hist-insight-chips" aria-label="Streaks">
          {insights.map((i) => (
            <span
              key={i.id}
              className={`hist-insight-chip insight-${i.kind}`}
            >
              <span className="hist-insight-chip-label">{i.label}</span>
              {i.value}
            </span>
          ))}
        </div>
      ) : null}
    </section>
  );
}
