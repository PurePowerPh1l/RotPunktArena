import { TRAINING_HISTORY_WINDOW } from "../training/comparison";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  EntryResultSummary,
  TrainingSessionDetail,
  TrainingSessionSummary,
  TrainingShooterOption,
} from "@rotpunktarena/domain";
import { trainingHistoryClearConfirmMessage } from "@rotpunktarena/domain";
import type { ShooterValue } from "../components/ShooterAutocomplete";
import type { ScoreDisplayMode } from "../components/TargetFace";
import { SlidingSeg } from "../components/SlidingSeg";
import { ShooterFilterBar } from "../components/ShooterFilterBar";
import { TrainingTrendChart } from "../components/TrainingTrendChart";
import { IconTraining, IconTrophy } from "../components/UiIcons";
import { ExpandSlot } from "../components/ExpandSlot";
import { usePrintHotkey } from "../hooks/usePrintHotkey";
import { confirmDialog } from "../hooks/useAppDialog";
import { evaluateAchievements } from "../training/achievements";
import {
  evaluateGoals,
  loadGoals,
  saveGoals,
  type TrainingGoal,
} from "../training/goals";
import {
  computeCompareBanner,
  computeFormInsights,
  filterSessionsByWindow,
  HISTORY_WINDOW_OPTIONS,
  type HistoryWindowDays,
} from "../training/insights";
import { leagueFromSessions, rankFromSr } from "../training/league";
import { computeSeriesPulse, applyLifetimeToPulse } from "../training/seriesPulse";
import {
  computeTrainingStats, levelFromXp,
  fmtDelta,
} from "../training/stats";
import { computeTransfer } from "../training/transfer";
import { printShotCard } from "../print/printShotCard";
import { createRequestSeq } from "../lib/requestSeq";
import * as api from "../api/commands";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { CompetitionHistoryPanel } from "./bureau/CompetitionHistoryPanel";
import { TrainingGoalsPanel } from "./training/TrainingGoalsPanel";
import { TrainingHeroPanel } from "./training/TrainingHeroPanel";
import { TrainingInsightsBar } from "./training/TrainingInsightsBar";
import { TrainingProgressPeek } from "./training/TrainingProgressPeek";
import { TrainingSeriesDetail } from "./training/TrainingSeriesDetail";
import { TrainingSeriesPanel } from "./training/TrainingSeriesPanel";
import { TrainingTransferPanel } from "./training/TrainingTransferPanel";

type Props = {
  defaultShooter?: ShooterValue;
};

type HistorySection = "training" | "competitions";
type FilterKey = "all" | string;
type WindowSeg = "7" | "30" | "90" | "all";

function filterKeyOf(o: {
  personId?: string | null;
  shooterName: string;
}): FilterKey {
  if (o.personId) return `id:${o.personId}`;
  return `name:${o.shooterName.trim().toLowerCase()}`;
}

function windowFromSeg(v: WindowSeg): HistoryWindowDays {
  if (v === "all") return null;
  return Number(v) as HistoryWindowDays;
}

function segFromWindow(v: HistoryWindowDays): WindowSeg {
  if (v == null) return "all";
  return String(v) as WindowSeg;
}

export function TrainingHistoryView({ defaultShooter }: Props) {
  const [section, setSection] = useState<HistorySection>("training");
  const [sessions, setSessions] = useState<TrainingSessionSummary[]>([]);
  const [lifetimes, setLifetimes] = useState<api.TrainingLifetime[]>([]);
  const [archive, setArchive] = useState<TrainingSessionSummary[]>([]);
  const [archiveOffset, setArchiveOffset] = useState(0);
  const [archiveBusy, setArchiveBusy] = useState(false);
  const [shooters, setShooters] = useState<TrainingShooterOption[]>([]);
  const [filter, setFilter] = useState<FilterKey>(() => {
    if (!defaultShooter?.name.trim()) return "all";
    if (defaultShooter.personId) return `id:${defaultShooter.personId}`;
    return `name:${defaultShooter.name.trim().toLowerCase()}`;
  });
  const [windowDays, setWindowDays] = useState<HistoryWindowDays>(30);
  const [metric, setMetric] = useState<ScoreDisplayMode>("punkte");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [achieveOpen, setAchieveOpen] = useState(false);
  const [progressMoreOpen, setProgressMoreOpen] = useState(false);
  const [goals, setGoals] = useState<TrainingGoal[]>(() => loadGoals(filter));
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  /** Kept during close animation; updated when a series loads. */
  const [detailView, setDetailView] = useState<TrainingSessionDetail | null>(
    null,
  );
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [latestDetail, setLatestDetail] = useState<TrainingSessionDetail | null>(null);
  const [latestDetailError, setLatestDetailError] = useState<string | null>(null);
  const [detailDisplayMode, setDetailDisplayMode] = useState<ScoreDisplayMode>("punkte");
  const [compBests, setCompBests] = useState<EntryResultSummary[]>([]);
  const [compLoading, setCompLoading] = useState(false);
  const loadSeq = useRef(createRequestSeq()).current;
  const detailSeq = useRef(createRequestSeq()).current;
  const compSeq = useRef(createRequestSeq()).current;
  const archiveSeq = useRef(createRequestSeq()).current;

  const { busy: actionBusy, run: runAction } = useAsyncAction();

  const apiFilter = useMemo(() => {
    if (filter === "all") return undefined;
    if (filter.startsWith("id:")) return { personId: filter.slice(3) };
    if (filter.startsWith("name:")) return { shooterName: filter.slice(5) };
    return undefined;
  }, [filter]);

  const personId = filter.startsWith("id:") ? filter.slice(3) : null;

  const refresh = useCallback(
    async (opts?: { refreshLeague?: boolean }) => {
      const token = loadSeq.begin();
      setLoading(true);
      try {
        setError(null);
        void opts;
        const [hist, optsList, allForLeague] = await Promise.all([
          api.listTrainingHistory(TRAINING_HISTORY_WINDOW, apiFilter),
          api.listTrainingShooters(),
          api.getTrainingLifetime(),
        ]);
        if (!loadSeq.isCurrent(token)) return;
        archiveSeq.begin(); setArchive([]); setArchiveOffset(0);
        setSessions(hist);
        setShooters(optsList);
        setLifetimes(allForLeague);
      } catch (e) {
        if (loadSeq.isCurrent(token)) setError(String(e));
      } finally {
        if (loadSeq.isCurrent(token)) setLoading(false);
      }
    },
    [apiFilter, loadSeq, archiveSeq],
  );

  useEffect(() => {
    if (section !== "training") return;
    void refresh();
  }, [refresh, section]);

  useEffect(() => {
    if (filter === "all" || shooters.length === 0) return;
    const exists = shooters.some((s) => filterKeyOf(s) === filter);
    if (!exists && filter.startsWith("name:")) setFilter("all");
  }, [shooters, filter]);

  useEffect(() => {
    setAchieveOpen(false);
    setProgressMoreOpen(false);
    setGoals(loadGoals(filter));
    setSelectedSessionId(null);
    setDetailView(null);
    setDetailError(null);
    setLatestDetail(null);
    setLatestDetailError(null);
  }, [filter]);

  useEffect(() => {
    try { saveGoals(filter, goals); } catch (e) { setError(`Trainingsziele konnten nicht gespeichert werden: ${String(e)}`); }
  }, [filter, goals]);

  /** Load competition bests for transfer panel (person-linked shooters only). */
  useEffect(() => {
    if (!personId || section !== "training") {
      setCompBests([]);
      return;
    }
    const token = compSeq.begin();
    setCompLoading(true);
    void (async () => {
      try {
        const comps = await api.listCompetitions(true);
        if (!compSeq.isCurrent(token)) return;
        const past = comps.filter(
          (c) => c.status === "closed" || c.status === "archived",
        );
        const recent = past.slice(0, 12);
        const batches = await Promise.all(
          recent.map((c) =>
            api.listCompetitionResults(c.id).catch(() => [] as EntryResultSummary[]),
          ),
        );
        if (!compSeq.isCurrent(token)) return;
        const mine = batches
          .flat()
          .filter((r) => r.personId === personId && r.shotCount > 0);
        setCompBests(mine);
      } catch {
        if (compSeq.isCurrent(token)) setCompBests([]);
      } finally {
        if (compSeq.isCurrent(token)) setCompLoading(false);
      }
    })();
  }, [personId, section, compSeq]);

  const openSession = useCallback(
    async (sessionId: string) => {
      if (selectedSessionId === sessionId) {
        setSelectedSessionId(null);
        setDetailError(null);
        return;
      }
      setSelectedSessionId(sessionId);
      setDetailError(null);
      const token = detailSeq.begin();
      setDetailLoading(true);
      try {
        const d = await api.getTrainingSessionDetail(sessionId);
        if (!detailSeq.isCurrent(token)) return;
        if (!d) {
          setDetailError("Serie nicht gefunden");
          return;
        }
        setDetailView(d);
      } catch (e) {
        if (!detailSeq.isCurrent(token)) return;
        setDetailError(String(e));
      } finally {
        if (detailSeq.isCurrent(token)) setDetailLoading(false);
      }
    },
    [detailSeq, selectedSessionId],
  );

  const closeDetail = useCallback(() => {
    setSelectedSessionId(null);
    setDetailError(null);
  }, []);

  const windowedSessions = useMemo(
    () => filterSessionsByWindow(sessions, windowDays),
    [sessions, windowDays],
  );

  const stats = useMemo(() => {
    const windowed = computeTrainingStats(windowedSessions);
    const lifetime = computeTrainingStats(sessions);
    const rows = filter === "all" ? lifetimes : lifetimes.filter((s) => s.key === filter);
    const xp = rows.reduce((total, s) => total + s.pointsTotal + s.shotCount * 1.5, 0);
    const level = levelFromXp(xp);
    Object.assign(lifetime, { xp, level:level.level, levelTitle:level.title, levelProgress:level.progress, xpIntoLevel:level.xpIntoLevel, xpForLevel:level.xpForLevel, xpToNext:level.xpToNext });
    return {
      ...windowed,
      level: lifetime.level,
      levelTitle: lifetime.levelTitle,
      levelProgress: lifetime.levelProgress,
      xp: lifetime.xp,
      xpIntoLevel: lifetime.xpIntoLevel,
      xpForLevel: lifetime.xpForLevel,
      xpToNext: lifetime.xpToNext,
    };
  }, [windowedSessions, sessions, lifetimes, filter]);
  const achievements = useMemo(
    () => evaluateAchievements(sessions),
    [sessions],
  );
  const leaguesByKey = useMemo(
    () => new Map(lifetimes.map((s) => [s.key, rankFromSr(s.sr, s.sessionCount)])),
    [lifetimes],
  );
  const league = useMemo(() => {
    if (filter === "all") return null;
    return leaguesByKey.get(filter) ?? leagueFromSessions(sessions);
  }, [filter, leaguesByKey, sessions]);
  const unlockedCount = useMemo(
    () => achievements.filter((a) => a.unlocked).length,
    [achievements],
  );
  const previewAchievements = useMemo(() => {
    const unlocked = achievements.filter((a) => a.unlocked);
    const locked = achievements.filter((a) => !a.unlocked);
    return [...unlocked, ...locked].slice(0, 6);
  }, [achievements]);
  const newestFirst = useMemo(
    () => [...windowedSessions].reverse(),
    [windowedSessions],
  );
  const lastPulse = useMemo(
    () => {
      const latest = windowedSessions[windowedSessions.length - 1];
      if (!latest) return null;
      const key = filterKeyOf(latest);
      return applyLifetimeToPulse(computeSeriesPulse(windowedSessions.filter((s) => filterKeyOf(s) === key)), lifetimes.find((s) => s.key === key));
    },
    [windowedSessions, lifetimes],
  );
  const goalProgress = useMemo(
    () => evaluateGoals(sessions, goals),
    [sessions, goals],
  );
  const topGoal = useMemo(() => {
    if (goalProgress.length === 0) return null;
    const open = goalProgress.find((g) => !g.done);
    return open ?? goalProgress[0] ?? null;
  }, [goalProgress]);
  const compare = useMemo(
    () => computeCompareBanner(windowedSessions),
    [windowedSessions],
  );
  const insights = useMemo(
    () => computeFormInsights(windowedSessions, latestDetail?.shots ?? null),
    [windowedSessions, latestDetail],
  );
  const transfer = useMemo(() => {
    if (!personId) return null;
    return computeTransfer(windowedSessions, compBests);
  }, [personId, windowedSessions, compBests]);

  const chartAvg = metric === "teiler" ? stats.avgTeiler : stats.avgSeriePunkte;
  const trend =
    metric === "teiler"
      ? fmtDelta(stats.trendTeiler, true)
      : fmtDelta(stats.trendPunkte, false);
  const singleShooter = filter !== "all";

  const filterLabel =
    filter === "all"
      ? "Alle Schützen"
      : shooters.find((s) => filterKeyOf(s) === filter)?.shooterName ?? "Schütze";

  // Match the current selection by ID: detailView is retained for the close
  // animation and can still contain a previous series while a new one loads.
  const printDetail = selectedSessionId
    ? !detailLoading && detailView?.summary.id === selectedSessionId
      ? detailView
      : null
    : !loading && latestDetail?.summary.id === newestFirst[0]?.id
      ? latestDetail
      : null;

  const doPrint = useCallback(() => {
    if (section !== "training" || actionBusy || !printDetail?.shots.length) return;
    printShotCard({
      shooterName: printDetail.summary.shooterName,
      modeLabel: "Training",
      shots: printDetail.shots,
      seriesTotal: printDetail.summary.punkteTotal,
      maxShots: printDetail.summary.shotCount,
      displayMode: detailDisplayMode,
    });
  }, [section, actionBusy, printDetail, detailDisplayMode]);

  usePrintHotkey(section === "training" ? doPrint : null);

  const clearHistory = async () => {
    const ok = await confirmDialog({
      title: "Trainingsverlauf löschen?",
      body: trainingHistoryClearConfirmMessage(filterLabel),
      confirmLabel: "Löschen",
      danger: true,
      eyebrow: "Historie",
    });
    if (!ok) return;
    const result = await runAction(async () => {
      setError(null);
      await api.clearTrainingHistory(apiFilter);

      setDetailView(null);
      setSelectedSessionId(null);
      await refresh({ refreshLeague: true });
    });
    if (!result.ok && result.reason === "error" && result.message) {
      setError(result.message);
    }
  };

  const promoteToBureau = async () => {
    const selected = shooters.find((s) => filterKeyOf(s) === filter);
    if (!selected || selected.personId) return;
    const result = await runAction(async () => {
      setError(null);
      const promoted = await api.promoteTrainingShooter(selected.shooterName);

      await refresh({ refreshLeague: true });
      setFilter(`id:${promoted.person.id}`);
    });
    if (!result.ok && result.reason === "error" && result.message) {
      setError(result.message);
    }
  };

  const canPromote =
    filter.startsWith("name:") &&
    Boolean(shooters.find((s) => filterKeyOf(s) === filter && !s.personId));

  // Prefetch the last series for streak chips and synchronous system printing.
  useEffect(() => {
    const latestId = newestFirst[0]?.id;
    if (!latestId || loading) {
      setLatestDetail(null);
      setLatestDetailError(null);
      return;
    }
    let cancelled = false;
    setLatestDetailError(null);
    void (async () => {
      try {
        const d = await api.getTrainingSessionDetail(latestId);
        if (cancelled) return;
        setLatestDetail(d);
        if (!d) setLatestDetailError("Letzte Serie nicht gefunden");
      } catch (e) {
        if (cancelled) return;
        setLatestDetail(null);
        setLatestDetailError(`Schussdaten der letzten Serie konnten nicht geladen werden: ${String(e)}`);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [newestFirst[0]?.id, loading]);

  return (
    <div className="training-hist">
      <div className="hist-section-bar">
        <SlidingSeg
          ariaLabel="Statistik-Bereich"
          value={section}
          onChange={setSection}
          options={[
            {
              value: "training",
              label: (
                <span className="seg-label">
                  <IconTraining size={14} /> Training
                </span>
              ),
            },
            {
              value: "competitions",
              label: (
                <span className="seg-label">
                  <IconTrophy size={14} /> Wettkämpfe
                </span>
              ),
            },
          ]}
        />
      </div>

      {section === "competitions" ? (
        <CompetitionHistoryPanel />
      ) : (
        <div className="hist-training-wrap">
          {error ? <p className="banner-error">{error}</p> : null}

          <p className="hint">Auswertung der zuletzt geladenen maximal {TRAINING_HISTORY_WINDOW} Serien im gewählten Zeitraum; Liga je Schütze im zuletzt geladenen Gesamtfenster. Punktevergleiche auf zehn Schüsse normiert. Keine Gesamtstatistik über die vollständige Historie.</p>
          <div className="hist-filter-row">
            <ShooterFilterBar
              shooters={shooters}
              filter={filter}
              onChange={setFilter}
              filterKeyOf={filterKeyOf}
              leagueOf={(key) => leaguesByKey.get(key)}
            />
            <SlidingSeg
              size="sm"
              ariaLabel="Zeitfenster"
              value={segFromWindow(windowDays)}
              onChange={(v) => setWindowDays(windowFromSeg(v))}
              options={HISTORY_WINDOW_OPTIONS.map((o) => ({
                value: segFromWindow(o.value),
                label: o.label,
              }))}
            />
          </div>

          {canPromote ? (
            <div className="hist-promote-row">
              <p className="hint">
                „{filterLabel}“ ist nur Freitext — noch nicht in der Büro-Personenliste.
              </p>
              <button
                type="button"
                disabled={actionBusy}
                onClick={() => void promoteToBureau()}
              >
                In Büro anlegen
              </button>
            </div>
          ) : null}

          <div className="training-body">
            <TrainingInsightsBar compare={compare} insights={insights} />

            <section className="panel trend-panel hist-primary">
              <div className="trend-head">
                <h2>
                  <IconTraining size={18} /> Leistungsverlauf
                </h2>
                <SlidingSeg
                  size="sm"
                  ariaLabel="Kennzahl"
                  value={metric}
                  onChange={setMetric}
                  options={[
                    { value: "punkte", label: "Punkte / 10 Schüsse" },
                    { value: "teiler", label: "Ø Teiler" },
                  ]}
                />
              </div>
              {loading ? (
                <p className="hint">Laden…</p>
              ) : (
                <TrainingTrendChart
                  sessions={windowedSessions}
                  metric={metric}
                  average={chartAvg}
                  onSelectSession={(id) => void openSession(id)}
                />
              )}
            </section>

            <ExpandSlot
              open={Boolean(selectedSessionId)}
              scrollOnOpen
              className="hist-detail-slot"
              onExited={() => {
                if (!selectedSessionId) setDetailView(null);
              }}
            >
              <TrainingSeriesDetail
                detail={detailView}
                loading={detailLoading || actionBusy || detailView?.summary.id !== selectedSessionId}
                displayMode={detailDisplayMode}
                onDisplayModeChange={setDetailDisplayMode}
                onPrint={doPrint}
                onClose={closeDetail}
              />
            </ExpandSlot>

            <TrainingSeriesPanel
              sessions={windowedSessions}
              newestFirst={newestFirst}
              loading={loading}
              busy={actionBusy}
              printDisabled={!printDetail?.shots.length}
              bestSerie={stats.bestSerie}
              sessionCount={stats.sessionCount}
              lastPulse={lastPulse}
              selectedId={selectedSessionId}
              detailLoading={detailLoading && !detailView}
              detailError={selectedSessionId ? detailError : latestDetailError}
              onSelect={(id) => void openSession(id)}
              onPrint={doPrint}
              onClear={() => void clearHistory()}
              onRefresh={() => void refresh()}
            />

            <section className="panel">
              <h2>Gesamter Trainingsbestand</h2>
              <p>{lifetimes.filter((s) => filter === "all" || s.key === filter).reduce((n,s) => n+s.sessionCount,0)} Serien � {lifetimes.filter((s) => filter === "all" || s.key === filter).reduce((n,s) => n+s.shotCount,0)} Sch�sse insgesamt. Trends und Erfolge zeigen weiterhin das aktuelle 200er-Fenster.</p>
              <button className="btn ghost" disabled={archiveBusy || loading} onClick={() => void (async () => {
                const token = archiveSeq.begin();
                setArchiveBusy(true);
                try {
                  const page = await api.listTrainingHistory(200, apiFilter, archiveOffset);
                  if (!archiveSeq.isCurrent(token)) return;
                  setArchive(page); setArchiveOffset(archiveOffset + page.length);
                } catch (e) { setError(String(e)); } finally { setArchiveBusy(false); }
              })()}>N�chste 200 Serien im Archiv</button>
              {archiveOffset > 0 ? <button className="btn ghost" onClick={() => {archiveSeq.begin();setArchiveOffset(0);setArchive([]);}}>Archiv zur�cksetzen</button> : null}
              {archive.length ? <div className="shot-list"><table><thead><tr><th>Datum</th><th>Sch�tze</th><th>Sch�sse</th><th>Punkte</th></tr></thead><tbody>{[...archive].reverse().map((s) => <tr key={s.id}><td>{new Date(s.endedAt).toLocaleString("de-DE")}</td><td><button className="btn ghost" onClick={() => void openSession(s.id)}>{s.shooterName}</button></td><td>{s.shotCount}</td><td>{s.punkteTotal.toFixed(1)}</td></tr>)}</tbody></table></div> : null}
            </section>

            <section className="panel hist-progress">
              <div className="hist-progress-head">
                <h2>
                  <IconTrophy size={16} /> Fortschritt
                </h2>
              </div>

              <TrainingProgressPeek
                filterLabel={filterLabel}
                singleShooter={singleShooter}
                stats={stats}
                league={league}
                trend={trend}
                topGoal={singleShooter ? topGoal : null}
              />

              <div className="hist-progress-more">
                <button
                  type="button"
                  className="hist-progress-summary"
                  aria-expanded={progressMoreOpen}
                  onClick={() => setProgressMoreOpen((v) => !v)}
                >
                  <span className="hist-progress-summary-main">
                    {progressMoreOpen ? "Weniger anzeigen" : "Mehr anzeigen"}
                    {!progressMoreOpen ? (
                      <span className="hist-progress-summary-meta">
                        Achievements, Ziele
                        {personId ? ", Match-Vergleich" : ""}
                      </span>
                    ) : null}
                  </span>
                  <span
                    className={`hist-progress-chevron${progressMoreOpen ? " is-open" : ""}`}
                    aria-hidden
                  />
                </button>

                <ExpandSlot
                  open={progressMoreOpen}
                  className="hist-progress-expand"
                >
                  <div className="hist-progress-body">
                    <TrainingHeroPanel
                      filterLabel={filterLabel}
                      singleShooter={singleShooter}
                      stats={stats}
                      league={league}
                      trend={trend}
                      achievements={achievements}
                      previewAchievements={previewAchievements}
                      unlockedCount={unlockedCount}
                      achieveOpen={achieveOpen}
                      onAchieveOpenChange={setAchieveOpen}
                    />

                    {singleShooter ? (
                      <TrainingGoalsPanel
                        sessions={sessions}
                        goals={goals}
                        onChange={setGoals}
                        disabled={actionBusy}
                      />
                    ) : null}

                    {personId ? (
                      <TrainingTransferPanel
                        transfer={transfer}
                        loading={compLoading}
                      />
                    ) : null}
                  </div>
                </ExpandSlot>
              </div>
            </section>
          </div>
        </div>
      )}
    </div>
  );
}
