/**
 * Arena orchestrator: session state, sound, and layout.
 * UI sections live in ./live/* (score column, session controls, competition picker).
 * Top chrome lives in App (persistent across place switches).
 */
import { useEffect, useState } from "react";
import { SlidingSeg } from "../components/SlidingSeg";
import {
  TargetFace,
  type FaceLabelMode,
  type ScoreDisplayMode,
} from "../components/TargetFace";
import type { ShooterValue } from "../components/ShooterAutocomplete";
import { MomentFlash } from "../components/live/MomentFlash";
import { SeriesCeremony } from "../components/live/SeriesCeremony";
import { TrainingProgressStrip } from "../components/live/TrainingProgressStrip";
import { StreakChip } from "../components/live/StreakChip";
import { useLiveSession } from "../hooks/useLiveSession";
import { useTrainingArenaProgress } from "../hooks/useTrainingArenaProgress";
import { bestShotOf, useScoreDisplay } from "../hooks/useScoreDisplay";
import { useTrainingMoments } from "../hooks/useTrainingMoments";
import {
  type PlayShotOpts,
} from "../live/presenceContract";
import { formatPersonName } from "../lib/format";
import { xpPreviewForLiveSeries } from "../training/seriesPulse";
import * as api from "../api/commands";
import type { CompetitionCreateInput } from "./bureau/CompetitionCreateForm";
import type { ArenaHandoff } from "./bureau/StartListPanel";
import { LiveScoreColumn } from "./live/LiveScoreColumn";
import { LiveSessionControls } from "./live/LiveSessionControls";
import {
  resolveLiveHardwareStart,
  serialAvailabilityFromLive,
} from "./live/preferLiveSimulator";
import { useLiveLinkStatus } from "../hooks/useLiveLinkStatus";
import { useRedDotSheets } from "../hooks/useRedDotSheets";
import { useArenaActiveCompetitions } from "../hooks/useArenaActiveCompetitions";
import { useCompetitionRoster } from "../hooks/useCompetitionRoster";
import { useLiveTrainingPresence } from "../hooks/useLiveTrainingPresence";
import { RedDotSetupSheet } from "../components/RedDotSetupSheet";
import { RedDotWakeSheet } from "../components/RedDotWakeSheet";

type Props = {
  trainingShooter: ShooterValue;
  onTrainingShooterChange: (next: ShooterValue) => void;
  playShot: (opts?: PlayShotOpts) => void;
  adminMode?: boolean;
  /** Permission for mouse-aim / simulator gate — independent of sheet open. */
  developerMode?: boolean;
  /** Dev sheet open (UI only) — simulator fire helpers in controls. */
  developerSheetOpen?: boolean;
  /** Mouse-aim preference from Developer sheet — effect still needs developerMode. */
  mouseAimEnabled?: boolean;
  /** Bumped from App after DevPanel test-shot inject. */
  devShotInjectEpoch?: number;
  onOpenHistory: () => void;
  onArenaModeChange?: (mode: "training" | "competition") => void;
  /** Competition has Nachkauf enabled (top-bar gold chip). */
  onNachkaufActiveChange?: (active: boolean) => void;
  /** True while Arena place is visible — triggers list reload. */
  arenaVisible?: boolean;
  /** Bumped when Verwaltung mutates competitions — Arena reloads active list. */
  competitionsEpoch?: number;
  /** Arena selection (active only). */
  competitionId: string;
  onCompetitionIdChange: (id: string) => void;
  /** Newly created drafts belong in Verwaltung only — do not overwrite Arena. */
  onBureauCompetitionIdChange?: (id: string | null) => void;
  arenaHandoff?: ArenaHandoff | null;
  onArenaHandoffConsumed?: () => void;
  /** Bumped from top-bar badge to (re)open first-setup sheet. */
  setupRequestNonce?: number;
};

export function LiveStandView({
  trainingShooter,
  onTrainingShooterChange,
  playShot,
  adminMode = false,
  developerMode = false,
  developerSheetOpen = false,
  mouseAimEnabled = true,
  devShotInjectEpoch = 0,
  onOpenHistory,
  onArenaModeChange,
  onNachkaufActiveChange,
  arenaVisible = true,
  competitionsEpoch = 0,
  competitionId,
  onCompetitionIdChange,
  onBureauCompetitionIdChange,
  arenaHandoff = null,
  onArenaHandoffConsumed,
  setupRequestNonce = 0,
}: Props) {
  const live = useLiveSession();
  const link = useLiveLinkStatus();
  const sheets = useRedDotSheets({
    link,
    setupRequestNonce,
  });
  const setCompetitionId = onCompetitionIdChange;
  const { competitions, reloadCompetitions } = useArenaActiveCompetitions({
    arenaVisible,
    competitionsEpoch,
    competitionId,
    onCompetitionIdChange: setCompetitionId,
  });
  const {
    entries,
    entryId,
    setEntryId,
    teams,
    teamId,
    setTeamId,
    refreshEntries,
    refreshTeams,
    teamScoringEnabled,
    selectedComp,
    selectedEntry,
    nachkaufEnabled,
  } = useCompetitionRoster({
    competitionId,
    competitions,
    running: live.running,
  });
  const [mode, setMode] = useState<"training" | "competition">("training");
  const [displayMode, setDisplayMode] = useState<ScoreDisplayMode>("punkte");
  const [faceLabels, setFaceLabels] = useState<FaceLabelMode>("value");
  const [endlessMode, setEndlessMode] = useState(false);
  const [createBusy, setCreateBusy] = useState(false);
  const [stripFocus, setStripFocus] = useState<"xp" | "liga">("xp");

  useEffect(() => {
    onArenaModeChange?.(mode);
  }, [mode, onArenaModeChange]);

  useEffect(() => {
    if (devShotInjectEpoch === 0) return;
    void live.refresh();
    // Only react to inject epoch — refresh identity is stable enough for this bump.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [devShotInjectEpoch]);

  const shooter = trainingShooter.name;

  useEffect(() => {
    if (!arenaHandoff) return;
    let cancelled = false;
    const { competitionId: cid, entryId: eid } = arenaHandoff;
    setMode("competition");
    void (async () => {
      const list = await reloadCompetitions();
      if (cancelled) return;
      const active = list.find((c) => c.id === cid && c.status === "active");
      if (!active) {
        onArenaHandoffConsumed?.();
        return;
      }
      setCompetitionId(cid);
      const entriesList = await refreshEntries(cid);
      if (cancelled) return;
      if (entriesList.some((e) => e.id === eid)) setEntryId(eid);
      onArenaHandoffConsumed?.();
    })();
    return () => {
      cancelled = true;
    };
  }, [arenaHandoff]);

  useEffect(() => {
    if (live.running) return;
    const c = competitions.find((x) => x.id === competitionId);
    if (c?.scoringMode === "teiler") setDisplayMode("teiler");
    else if (mode === "competition") setDisplayMode("punkte");
  }, [competitionId, competitions, mode, live.running]);

  useEffect(() => {
    onNachkaufActiveChange?.(mode === "competition" && nachkaufEnabled);
  }, [mode, nachkaufEnabled, onNachkaufActiveChange]);

  const last = live.state?.lastShot ?? null;
  const shots = live.state?.shots ?? [];
  const sessionOpen = Boolean(live.state?.session && !live.state.session.endedAt);
  const maxShots = live.state?.maxShots ?? null;
  const shotCount = shots.length;
  const seriesComplete = Boolean(live.state?.seriesComplete);
  /** Best shot only after series end (Punkte = max, Teiler = min). */
  const bestShot =
    seriesComplete && shots.length > 0 ? bestShotOf(shots, displayMode) : null;
  const canInspect = seriesComplete && shotCount > 0;

  const trainingMode = mode === "training";
  const {
    scoreTick,
    ceremonyOpen,
    rivalEnabled,
    setRivalEnabled,
    focusShot,
    setFocusShot,
    clearPulseRef,
  } = useLiveTrainingPresence({
    trainingMode,
    shots,
    last,
    displayMode,
    seriesComplete,
    canInspect,
    playShot,
  });
  const moments = useTrainingMoments({
    enabled: trainingMode,
    shots,
    last,
    displayMode,
  });
  const arenaProgress = useTrainingArenaProgress({
    enabled: trainingMode,
    shooter: trainingShooter,
    seriesComplete,
    trainingSave: live.state?.trainingSave,
    rivalEnabled,
  });
  clearPulseRef.current = arenaProgress.clearPulse;
  const canAim = sessionOpen && !seriesComplete && (maxShots == null || shotCount < maxShots);
  // Preference (checkbox) may stay true; effect requires developerMode. Sheet open ≠ feature on.
  const mouseAimActive = canAim && developerMode && mouseAimEnabled;
  const isTraining = mode === "training" && !live.state?.session?.competitionId;
  const setEndlessOnEngine = live.setEndlessMode;

  useEffect(() => {
    if (live.state?.endlessMode != null) {
      setEndlessMode(Boolean(live.state.endlessMode));
    }
  }, [live.state?.endlessMode]);

  useEffect(() => {
    if (mode !== "competition" || !endlessMode) return;
    setEndlessMode(false);
    void setEndlessOnEngine(false);
  }, [mode, endlessMode, setEndlessOnEngine]);

  const { primary, secondary, seriesPrimary } = useScoreDisplay(
    displayMode,
    last,
    live.state?.seriesTotal,
    live.state?.seriesTeilerTotal,
  );

  const createCompetition = async (input: CompetitionCreateInput): Promise<boolean> => {
    if (createBusy) return false;
    setCreateBusy(true);
    try {
      const { activateOnCreate, ...createInput } = input;
      const created = await api.createCompetition(createInput);
      if (activateOnCreate) {
        try {
          await api.setCompetitionStatus(created.id, "active");
        } catch (e) {
          window.alert(
            `Wettkampf angelegt, aber Status konnte nicht auf Aktiv gesetzt werden:\n${String(e)}`,
          );
          await reloadCompetitions();
          onBureauCompetitionIdChange?.(created.id);
          return true;
        }
        await reloadCompetitions();
        onCompetitionIdChange(created.id);
        onBureauCompetitionIdChange?.(created.id);
        return true;
      }
      await reloadCompetitions();
      // Draft stays Verwaltung-only; Arena keeps last active selection.
      onBureauCompetitionIdChange?.(created.id);
      return true;
    } catch (e) {
      window.alert(`Wettkampf konnte nicht angelegt werden:\n${String(e)}`);
      return false;
    } finally {
      setCreateBusy(false);
    }
  };

  /** Add or select shooter as active starter; in team mode also assign to the team. */
  const ensureStarter = async (shooterValue: ShooterValue): Promise<boolean> => {
    if (!competitionId) return false;
    const trimmed = shooterValue.name.trim();
    if (!trimmed && !shooterValue.personId) return false;

    let personId = shooterValue.personId;
    if (!personId) {
      const promoted = await api.promoteTrainingShooter(trimmed);
      personId = promoted.person.id;
    }

    const list = await api.listEntries(competitionId);
    let entry = list.find((e) => e.personId === personId) ?? null;
    if (!entry) {
      try {
        entry = await api.addEntry(competitionId, personId);
      } catch (e) {
        const msg = String(e);
        if (msg.includes("bereits in der Startliste")) {
          const refreshed = await api.listEntries(competitionId);
          entry = refreshed.find((e) => e.personId === personId) ?? null;
        } else {
          throw e;
        }
      }
    }
    if (!entry) return false;

    if (teamScoringEnabled && teamId) {
      const team = teams.find((t) => t.id === teamId);
      if (team && !team.memberEntryIds.includes(entry.id)) {
        await api.addTeamMember(teamId, entry.id);
        await refreshTeams(competitionId);
      }
    }

    await refreshEntries(competitionId);
    setEntryId(entry.id);
    return true;
  };

  /** Create global team (or select if name already exists). */
  const ensureTeam = async (name: string): Promise<boolean> => {
    if (!teamScoringEnabled) return false;
    const trimmed = name.trim();
    if (!trimmed) return false;
    const existing = teams.find(
      (t) => t.name.trim().toLowerCase() === trimmed.toLowerCase(),
    );
    if (existing) {
      setTeamId(existing.id);
      return true;
    }
    try {
      const created = await api.createTeam(trimmed);
      await refreshTeams(competitionId || null);
      setTeamId(created.id);
      return true;
    } catch (e) {
      window.alert(`Team konnte nicht angelegt werden:\n${String(e)}`);
      return false;
    }
  };

  const handleModeChange = (next: "training" | "competition") => {
    if (next === mode) return;
    if (next === "training" && selectedEntry) {
      onTrainingShooterChange({
        name: formatPersonName(selectedEntry.lastName, selectedEntry.firstName, ""),
        personId: selectedEntry.personId,
      });
    }
    setMode(next);
    if (
      next !== "competition" ||
      !competitionId ||
      (!trainingShooter.personId && !trainingShooter.name.trim())
    ) {
      return;
    }

    void (async () => {
      try {
        if (selectedComp?.teamScoringEnabled) {
          let personId = trainingShooter.personId;
          if (!personId) {
            const promoted = await api.promoteTrainingShooter(
              trainingShooter.name.trim(),
            );
            personId = promoted.person.id;
          }
          const list = await refreshEntries(competitionId);
          const found = list.find((e) => e.personId === personId);
          if (!found) return;
          const teamList = await refreshTeams(competitionId || null);
          const team = teamList.find((t) => t.memberEntryIds.includes(found.id));
          if (team) setTeamId(team.id);
          setEntryId(found.id);
          return;
        }
        await ensureStarter(trainingShooter);
      } catch (e) {
        window.alert(`Starter konnte nicht übernommen werden:\n${String(e)}`);
      }
    })();
  };

  const start = async (useSimulator: boolean) => {
    if (mode === "competition" && entryId) {
      if (selectedEntry?.status === "done" && !nachkaufEnabled) return;
      await live.startEntryPrepared(entryId, useSimulator);
    } else {
      await live.startTraining(
        trainingShooter.name,
        useSimulator,
        trainingShooter.personId,
        endlessMode,
      );
    }
  };

  const onEndlessModeChange = (next: boolean) => {
    setEndlessMode(next);
    void live.setEndlessMode(next);
  };

  const startNext = async () => {
    if (live.busy) return;
    const decision = resolveLiveHardwareStart({
      serial: serialAvailabilityFromLive(live.state?.serialFeature),
      linkReady: link.linked,
    });
    if (decision.action === "blocked") return;
    const pool =
      selectedComp?.teamScoringEnabled && teamId
        ? (() => {
            const memberIds = new Set(
              teams.find((t) => t.id === teamId)?.memberEntryIds ?? [],
            );
            return entries.filter((e) => memberIds.has(e.id));
          })()
        : entries;
    const next =
      pool.find(
        (e) =>
          e.id !== entryId &&
          e.status !== "done" &&
          (e.status === "waiting" || e.status === "probe"),
      ) ??
      pool.find((e) => e.status === "waiting" || e.status === "probe");
    if (!next || next.status === "done") return;
    setEntryId(next.id);
    await live.stopThenStartEntry(next.id, decision.useSimulator);
  };

  const entryBlocked =
    mode === "competition" &&
    Boolean(
      selectedEntry &&
        selectedEntry.status === "done" &&
        !nachkaufEnabled,
    );
  const startNachkauf =
    mode === "competition" &&
    Boolean(selectedEntry?.status === "done" && nachkaufEnabled);

  const canStartCompetition =
    mode === "competition"
      ? Boolean(entryId) && !entryBlocked
      : trainingShooter.name.trim().length > 0;

  return (
    <div className="stand">
      <main className="stage">
        <LiveScoreColumn
          shooterFallback={shooter}
          sessionShooterName={live.state?.session?.shooterName}
          selectedEntry={selectedEntry}
          displayMode={displayMode}
          onDisplayModeChange={setDisplayMode}
          scoreTick={scoreTick}
          primary={primary}
          secondary={secondary}
          seriesPrimary={seriesPrimary}
          seriesTotalPunkte={live.state?.seriesTotal ?? 0}
          last={last}
          best={bestShot}
          focusShot={canInspect ? focusShot : null}
          onFocusShot={canInspect ? setFocusShot : undefined}
          shots={shots}
          maxShots={maxShots}
          shotCount={shotCount}
          seriesComplete={seriesComplete}
          detail={live.detail}
          mode={mode}
          nachkaufEnabled={nachkaufEnabled}
          nachkaufPurchased={selectedEntry?.nachkaufPurchased ?? 0}
          endlessMode={endlessMode}
          onEndlessModeChange={onEndlessModeChange}
          rivalTarget={trainingMode ? arenaProgress.rivalTarget : null}
          ceremony={
            trainingMode ? (
              <SeriesCeremony
                open={ceremonyOpen}
                seriesTotal={seriesPrimary}
                shotCount={shotCount}
                maxShots={maxShots}
                pulse={arenaProgress.pulse}
                xpPreview={
                  arenaProgress.pulse
                    ? null
                    : xpPreviewForLiveSeries(
                        live.state?.seriesTotal ?? 0,
                        shotCount,
                      )
                }
                onOpenStats={onOpenHistory}
              />
            ) : null
          }
        />

        <section className="face-col">
          <div className={`face-wrap${mouseAimActive ? " face-wrap-interactive" : ""}`}>
            <TargetFace
              shots={shots}
              last={last}
              best={bestShot}
              focusShot={canInspect ? focusShot : null}
              onFocusShot={canInspect ? setFocusShot : undefined}
              interactive={mouseAimActive}
              onAimClick={(x, y) => void live.fireAt(x, y)}
              labelMode={faceLabels}
              displayMode={displayMode}
              allowInspect={canInspect}
            />
            {trainingMode ? (
              <MomentFlash
                flashKind={moments.flashKind}
                toast={moments.toast}
                onDismissToast={moments.clearToast}
              />
            ) : null}
          </div>
          <div className="face-tools">
            <SlidingSeg
              size="sm"
              ariaLabel="Beschriftung"
              value={faceLabels}
              onChange={setFaceLabels}
              options={[
                { value: "value", label: "Wert" },
                { value: "index", label: "Nr." },
                { value: "off", label: "Aus" },
              ]}
            />
            <StreakChip shots={shots} visible={trainingMode} />
            {mouseAimActive ? <p className="face-hint">Klick = Schuss</p> : null}
            {seriesComplete && shotCount > 0 ? (
              <p className="face-hint">
                Schuss antippen · Mausrad zoomen · ziehen · Doppelklick zurück
              </p>
            ) : null}
          </div>
        </section>
      </main>

      <LiveSessionControls
        mode={mode}
        onModeChange={handleModeChange}
        running={live.running}
        busy={live.busy}
        sessionOpen={sessionOpen}
        canAim={canAim}
        canStartCompetition={canStartCompetition}
        isTraining={isTraining}
        endlessMode={endlessMode}
        shotCount={shotCount}
        serialFeature={live.state?.serialFeature}
        transport={live.state?.transport}
        autoFire={live.state?.autoFire}
        hasSession={Boolean(live.state?.session)}
        trainingShooter={trainingShooter}
        onTrainingShooterChange={onTrainingShooterChange}
        competitions={competitions}
        competitionId={competitionId}
        onCompetitionIdChange={setCompetitionId}
        entries={entries}
        entryId={entryId}
        onEntryIdChange={setEntryId}
        teams={teams}
        teamId={teamId}
        onTeamIdChange={setTeamId}
        selectedComp={selectedComp}
        entryBlocked={entryBlocked}
        startNachkauf={startNachkauf}
        createBusy={createBusy}
        onCreateCompetition={createCompetition}
        onEnsureStarter={ensureStarter}
        onEnsureTeam={ensureTeam}
        onStart={(sim) => void start(sim)}
        linkReady={link.linked}
        onStop={() => void live.stop()}
        onStartNext={() => void startNext()}
        onReset={() => void live.resetSeries()}
        onFireOnce={() => void live.fireOnce()}
        onToggleAuto={() => void live.toggleAuto()}
        adminMode={adminMode}
        developerMode={developerMode}
        devOpen={developerSheetOpen}
        progressStrip={
          trainingMode ? (
            <TrainingProgressStrip
              progress={arenaProgress.progress}
              focus={stripFocus}
              onFocusChange={setStripFocus}
              rivalEnabled={rivalEnabled}
              onRivalEnabledChange={setRivalEnabled}
              rivalTarget={arenaProgress.rivalTarget}
            />
          ) : null
        }
      />
      <RedDotSetupSheet
        open={sheets.setupOpen}
        onClose={sheets.closeSetup}
        onLinked={sheets.linkedSetup}
      />
      <RedDotWakeSheet
        open={sheets.wakeSheetOpen}
        targetName={link.targetName}
        rfcommStatus={link.rfcommStatus}
        reason={link.reason}
        onClose={sheets.closeWake}
        onLinked={sheets.linkedWake}
      />
      {sheets.showSetupReopen ? (
        <button
          type="button"
          className="reddot-setup-reopen"
          onClick={sheets.reopenSetup}
        >
          RedDot einrichten
        </button>
      ) : null}
      {sheets.showWakeReopen ? (
        <button
          type="button"
          className="reddot-setup-reopen"
          onClick={sheets.reopenWake}
        >
          Verbinden
        </button>
      ) : null}
    </div>
  );
}
