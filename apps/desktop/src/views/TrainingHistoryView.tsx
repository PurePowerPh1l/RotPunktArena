import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  EntryResultSummary,
  TrainingSessionDetail,
  TrainingSessionSummary,
  TrainingShooterOption,
} from "@reddot/domain";
import { trainingHistoryClearConfirmMessage } from "@reddot/domain";
import type { ShooterValue } from "../components/ShooterAutocomplete";
import type { ScoreDisplayMode } from "../components/TargetFace";
import { SlidingSeg } from "../components/SlidingSeg";
import { ShooterFilterBar } from "../components/ShooterFilterBar";
import { TrainingTrendChart } from "../components/TrainingTrendChart";
import { IconTraining, IconTrophy } from "../components/UiIcons";
import { ExpandSlot } from "../components/ExpandSlot";
import { usePrintHotkey } from "../hooks/usePrintHotkey";
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
import { leagueFromSessions, leagueMapFromSessions } from "../training/league";
import { computeSeriesPulse } from "../training/seriesPulse";
import {
  computeTrainingStats,
  fmtDelta,
} from "../training/stats";
import { computeTransfer } from "../training/transfer";
import { printTrainingHistorySheet } from "../print/printSheets";
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
  const [leagueSessions, setLeagueSessions] = useState<TrainingSessionSummary[]>(
    [],
  );
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
  const [latestShots, setLatestShots] = useState<
    TrainingSessionDetail["shots"] | null
  >(null);
  const [compBests, setCompBests] = useState<EntryResultSummary[]>([]);
  const [compLoading, setCompLoading] = useState(false);
  const loadSeq = useRef(createRequestSeq()).current;
  const detailSeq = useRef(createRequestSeq()).current;
  const compSeq = useRef(createRequestSeq()).current;
  const leagueCache = useRef<TrainingSessionSummary[] | null>(null);
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
        const needLeague = opts?.refreshLeague || !leagueCache.current;
        const [hist, optsList, allForLeague] = await Promise.all([
          api.listTrainingHistory(200, apiFilter),
          api.listTrainingShooters(),
          needLeague
            ? api.listTrainingHistory(200)
            : Promise.resolve(leagueCache.current!),
        ]);
        if (!loadSeq.isCurrent(token)) return;
        if (needLeague) leagueCache.current = allForLeague;
        setSessions(hist);
        setShooters(optsList);
        setLeagueSessions(allForLeague);
      } catch (e) {
        if (loadSeq.isCurrent(token)) setError(String(e));
      } finally {
        if (loadSeq.isCurrent(token)) setLoading(false);
      }
    },
    [apiFilter, loadSeq],
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
    setLatestShots(null);
  }, [filter]);

  useEffect(() => {
    saveGoals(filter, goals);
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
  }, [windowedSessions, sessions]);
  const achievements = useMemo(
    () => evaluateAchievements(sessions),
    [sessions],
  );
  const leaguesByKey = useMemo(
    () => leagueMapFromSessions(leagueSessions, filterKeyOf),
    [leagueSessions],
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
    () => computeSeriesPulse(windowedSessions),
    [windowedSessions],
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
    () => computeFormInsights(windowedSessions, latestShots),
    [windowedSessions, latestShots],
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

  const doPrint = useCallback(() => {
    if (section !== "training" || windowedSessions.length === 0) return;
    printTrainingHistorySheet({
      title: "Trainingshistorie",
      filterLabel,
      sessions: windowedSessions,
    });
  }, [filterLabel, section, windowedSessions]);

  usePrintHotkey(section === "training" ? doPrint : null);

  const clearHistory = async () => {
    const ok = window.confirm(trainingHistoryClearConfirmMessage(filterLabel));
    if (!ok) return;
    const result = await runAction(async () => {
      setError(null);
      await api.clearTrainingHistory(apiFilter);
      leagueCache.current = null;
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
      leagueCache.current = null;
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

  // Prefetch last series shots for streak chips.
  useEffect(() => {
    const latestId = newestFirst[0]?.id;
    if (!latestId || loading) {
      setLatestShots(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const d = await api.getTrainingSessionDetail(latestId);
        if (cancelled) return;
        setLatestShots(d?.shots ?? null);
      } catch {
        if (!cancelled) setLatestShots(null);
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
                    { value: "punkte", label: "Σ Punkte" },
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
                loading={detailLoading}
                onClose={closeDetail}
              />
            </ExpandSlot>

            <TrainingSeriesPanel
              sessions={windowedSessions}
              newestFirst={newestFirst}
              loading={loading}
              busy={actionBusy}
              bestSerie={stats.bestSerie}
              sessionCount={stats.sessionCount}
              lastPulse={lastPulse}
              selectedId={selectedSessionId}
              detailLoading={detailLoading && !detailView}
              detailError={detailError}
              onSelect={(id) => void openSession(id)}
              onPrint={doPrint}
              onClear={() => void clearHistory()}
              onRefresh={() => void refresh()}
            />

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
