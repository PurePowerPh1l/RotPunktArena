import type { AchievementStatus } from "../../training/achievements";
import type { LeagueRank } from "../../training/league";
import type { TrainingStats } from "../../training/stats";
import { fmtStat } from "../../training/stats";
import { LeagueBadge } from "../../components/LeagueBadge";
import { IconCheck, IconTrophy } from "../../components/UiIcons";

type Trend = { kind: string; text: string };

type Props = {
  filterLabel: string;
  singleShooter: boolean;
  stats: TrainingStats;
  league: LeagueRank | null;
  trend: Trend;
  achievements: AchievementStatus[];
  previewAchievements: AchievementStatus[];
  unlockedCount: number;
  achieveOpen: boolean;
  onAchieveOpenChange: (open: boolean) => void;
};

export function TrainingHeroPanel({
  filterLabel,
  singleShooter,
  stats,
  league,
  trend,
  achievements,
  previewAchievements,
  unlockedCount,
  achieveOpen,
  onAchieveOpenChange,
}: Props) {
  return (
    <section className="panel train-hero" aria-label="Fortschritt">
      <div className="train-level">
        <div className="train-level-badge" aria-hidden>
          <IconTrophy size={22} />
          <span>{stats.level}</span>
        </div>
        <div className="train-level-body">
          <p className="train-level-kicker">{filterLabel}</p>
          <h2 className="train-level-title">
            {singleShooter ? stats.levelTitle : "Übersicht"}
            {singleShooter ? (
              <span className="train-level-sub"> Level {stats.level}</span>
            ) : null}
          </h2>
          {singleShooter && league ? (
            <div className="train-league-row">
              <LeagueBadge rank={league} size="md" showLabel />
              {league.tier === "unranked" ? (
                <span className="train-league-meta">
                  noch {league.placementLeft} Serie
                  {league.placementLeft === 1 ? "" : "n"} bis Rang
                </span>
              ) : (
                <span className="train-league-meta">
                  {league.sr} SR
                  {league.progress < 1
                    ? ` · ${Math.round(league.progress * 100)}% zur nächsten Stufe`
                    : ""}
                </span>
              )}
            </div>
          ) : null}
          {singleShooter ? (
            <>
              <div
                className="train-xp-track"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(stats.levelProgress * 100)}
                aria-label="Fortschritt zum nächsten Level"
              >
                <div
                  className="train-xp-fill"
                  style={{ width: `${Math.round(stats.levelProgress * 100)}%` }}
                />
              </div>
              <p className="train-xp-meta">
                {fmtStat(stats.xpIntoLevel, 0)} / {fmtStat(stats.xpForLevel, 0)} XP
                {stats.xpToNext > 0
                  ? ` · noch ${fmtStat(stats.xpToNext, 0)} bis Level ${stats.level + 1}`
                  : " · Max-Stufe"}
              </p>
            </>
          ) : (
            <p className="train-xp-meta">
              Schütze wählen, um Level, Liga & Achievements zu sehen.
            </p>
          )}
        </div>
        <div
          className={`train-trend-pill trend-${trend.kind}`}
          title="Trend letzte Serien vs. davor"
        >
          <span className="train-trend-label">Trend</span>
          <strong>{trend.text}</strong>
        </div>
      </div>

      <div className="train-stat-grid">
        <article className="train-stat">
          <p className="train-stat-label">Ø Serie</p>
          <p className="train-stat-value">{fmtStat(stats.avgSeriePunkte)}</p>
          <p className="train-stat-hint">Punkte</p>
        </article>
        <article className="train-stat">
          <p className="train-stat-label">Ø / Schuss</p>
          <p className="train-stat-value">{fmtStat(stats.avgPunkteProSchuss)}</p>
          <p className="train-stat-hint">Punkte</p>
        </article>
        <article className="train-stat">
          <p className="train-stat-label">Ø Teiler</p>
          <p className="train-stat-value">{fmtStat(stats.avgTeiler)}</p>
          <p className="train-stat-hint">niedriger = besser</p>
        </article>
        <article className="train-stat">
          <p className="train-stat-label">Beste Serie</p>
          <p className="train-stat-value">{fmtStat(stats.bestSerie)}</p>
          <p className="train-stat-hint">Best-Teiler {fmtStat(stats.bestTeiler)}</p>
        </article>
        <article className="train-stat">
          <p className="train-stat-label">Serien</p>
          <p className="train-stat-value">{stats.sessionCount}</p>
          <p className="train-stat-hint">{stats.shotCount} Schüsse</p>
        </article>
        <article className="train-stat">
          <p className="train-stat-label">Letzte Serie</p>
          <p className="train-stat-value">
            {stats.lastSerie != null ? fmtStat(stats.lastSerie) : "—"}
          </p>
          <p className="train-stat-hint">Σ Punkte</p>
        </article>
      </div>

      {singleShooter ? (
        <div className="achieve-block">
          <div className="achieve-block-head">
            <p className="achieve-block-title">
              <IconTrophy size={14} /> Achievements
            </p>
            <p className="achieve-count">
              {unlockedCount}/{achievements.length}
            </p>
            <button
              type="button"
              className="achieve-toggle"
              aria-expanded={achieveOpen}
              onClick={() => onAchieveOpenChange(!achieveOpen)}
            >
              {achieveOpen ? "Weniger" : "Alle"}
            </button>
          </div>
          <div className={achieveOpen ? "achieve-grid is-expanded" : "achieve-grid"}>
            {(achieveOpen ? achievements : previewAchievements).map((a) => (
              <article
                key={a.id}
                className={a.unlocked ? "achieve-chip is-on" : "achieve-chip"}
                title={a.description}
              >
                <span className="achieve-chip-icon" aria-hidden>
                  {a.unlocked ? <IconCheck size={12} /> : <IconTrophy size={12} />}
                </span>
                <span className="achieve-chip-text">
                  <span className="achieve-chip-title">{a.title}</span>
                  {achieveOpen ? (
                    <span className="achieve-chip-desc">{a.description}</span>
                  ) : null}
                </span>
                {!a.unlocked && achieveOpen ? (
                  <span
                    className="achieve-track"
                    role="progressbar"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={Math.round(a.progress * 100)}
                    aria-label={`Fortschritt ${a.title}`}
                  >
                    <span
                      className="achieve-fill"
                      style={{ width: `${Math.round(a.progress * 100)}%` }}
                    />
                  </span>
                ) : null}
              </article>
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}
