import { useCallback, useEffect, useState } from "react";
import type { RecoverySessionInfo } from "@reddot/domain";
import { AppTopBar } from "./components/AppTopBar";
import type { AppView } from "./components/appNav";
import type { ShooterValue } from "./components/ShooterAutocomplete";
import { DevPanel } from "./components/DevPanel";
import { RecoveryGate } from "./components/RecoveryGate";
import { SettingsSheet } from "./components/SettingsSheet";
import { IconDev, IconMute, IconSettings, IconSound } from "./components/UiIcons";
import { useAppAccess } from "./hooks/useAppAccess";
import { useShotSound } from "./hooks/useShotSound";
import { useTrueFullscreen } from "./hooks/useTrueFullscreen";
import * as api from "./api/commands";
import { BureauView } from "./views/BureauView";
import type { ArenaHandoff } from "./views/bureau/StartListPanel";
import { LiveStandView } from "./views/LiveStandView";
import { TrainingHistoryView } from "./views/TrainingHistoryView";
import "./App.css";

const VIEW_SUBTITLE: Record<AppView, string> = {
  live: "Arena",
  history: "Statistik",
  bureau: "Verwaltung",
};

export default function App() {
  const fs = useTrueFullscreen();
  const sound = useShotSound();
  const access = useAppAccess();
  const {
    can,
    isAdminModeEnabled,
    isDeveloperModeEnabled,
    isDeveloperEntryVisible,
    adminAccessState,
    setAdminUnlockedForTests,
  } = access;
  const [view, setView] = useState<AppView>("live");
  const [arenaMode, setArenaMode] = useState<"training" | "competition">(
    "training",
  );
  const [arenaNachkauf, setArenaNachkauf] = useState(false);
  /** Verwaltung selection — any status (draft, active, …). */
  const [bureauCompetitionId, setBureauCompetitionId] = useState<string | null>(
    null,
  );
  /** Arena selection — only active competitions; cleared when demoted. */
  const [arenaCompetitionId, setArenaCompetitionId] = useState<string | null>(
    null,
  );
  /** Bumped when Verwaltung creates/updates/status-changes competitions → Arena reloads list. */
  const [competitionsEpoch, setCompetitionsEpoch] = useState(0);
  const [arenaHandoff, setArenaHandoff] = useState<ArenaHandoff | null>(null);
  /** Pure UI — opening a sheet never enables a mode. */
  const [isSettingsSheetOpen, setSettingsSheetOpen] = useState(false);
  const [isDeveloperSheetOpen, setDeveloperSheetOpen] = useState(false);
  const [mouseAimEnabled, setMouseAimEnabled] = useState(true);
  /** Bumped when DevPanel injects a test shot — LiveStandView refreshes. */
  const [devShotInjectEpoch, setDevShotInjectEpoch] = useState(0);
  /** Bumped when badge asks to open RedDot first-setup sheet. */
  const [setupRequestNonce, setSetupRequestNonce] = useState(0);
  const [trainingShooter, setTrainingShooter] = useState<ShooterValue>({
    name: "",
    personId: null,
  });
  const [recoverySessions, setRecoverySessions] = useState<
    RecoverySessionInfo[] | null
  >(null);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [list, lastShooters] = await Promise.all([
          api.listRecoverySessions(),
          api.listTrainingShooters().catch(() => []),
        ]);
        if (cancelled) return;
        setRecoverySessions(list);
        const last = lastShooters[0];
        if (last?.shooterName.trim()) {
          setTrainingShooter({
            name: last.shooterName,
            personId: last.personId ?? null,
          });
        }
      } catch (e) {
        if (!cancelled) {
          setRecoveryError(String(e));
          setRecoverySessions([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const onRecoveryResolved = useCallback(() => {
    setRecoverySessions([]);
  }, []);

  /** Arena pick → set Arena + mirror into Verwaltung. */
  const selectArenaCompetition = useCallback((id: string) => {
    const next = id.trim() ? id : null;
    setArenaCompetitionId(next);
    if (next) setBureauCompetitionId(next);
  }, []);

  /** Verwaltung pick — never forces Arena unless caller also activates. */
  const selectBureauCompetition = useCallback((id: string | null) => {
    setBureauCompetitionId(id);
  }, []);

  /** Verwaltung selected/activated an active competition → Arena follows. */
  const activateIntoArena = useCallback((id: string) => {
    setArenaCompetitionId(id);
  }, []);

  /** Verwaltung demoted this competition → drop it from Arena if selected. */
  const clearArenaIfCompetition = useCallback((id: string) => {
    setArenaCompetitionId((prev) => (prev === id ? null : prev));
  }, []);

  const invalidateCompetitions = useCallback(() => {
    setCompetitionsEpoch((n) => n + 1);
  }, []);

  const toggleSettingsSheet = useCallback(() => {
    setSettingsSheetOpen((open) => !open);
  }, []);

  const toggleDeveloperSheet = useCallback(() => {
    if (!can("developer:open")) return;
    setDeveloperSheetOpen((open) => !open);
  }, [can]);

  if (recoverySessions === null) {
    return (
      <div className="recovery-boot" aria-busy="true">
        <p>Prüfe offene Sessions…</p>
        {recoveryError ? <p className="banner-error">{recoveryError}</p> : null}
      </div>
    );
  }

  if (recoverySessions.length > 0) {
    return (
      <RecoveryGate sessions={recoverySessions} onResolved={onRecoveryResolved} />
    );
  }

  return (
    <div
      className={`app-frame app-frame-${view}${isAdminModeEnabled ? " is-admin-mode" : ""}`}
    >
      <AppTopBar
        subtitle={VIEW_SUBTITLE[view]}
        view={view}
        nav={{
          onOpenLive: () => setView("live"),
          onOpenBureau: () => setView("bureau"),
          onOpenHistory: () => setView("history"),
        }}
        tools={
          <>
            <button
              type="button"
              className={`top-tool${sound.muted ? " is-on" : ""}`}
              onClick={sound.toggleMute}
              title={sound.muted ? "Ton an" : "Ton aus"}
              aria-label={sound.muted ? "Ton an" : "Ton aus"}
              aria-pressed={sound.muted}
            >
              {sound.muted ? <IconMute /> : <IconSound />}
            </button>
            <button
              type="button"
              className={`top-tool${isSettingsSheetOpen ? " is-on" : ""}`}
              onClick={toggleSettingsSheet}
              title="Einstellungen"
              aria-label="Einstellungen"
              aria-pressed={isSettingsSheetOpen}
            >
              <IconSettings />
            </button>
            {isDeveloperEntryVisible ? (
              <button
                type="button"
                className={`top-tool${isDeveloperSheetOpen ? " is-on" : ""}`}
                onClick={toggleDeveloperSheet}
                title="Entwickler"
                aria-label="Entwickler"
                aria-pressed={isDeveloperSheetOpen}
              >
                <IconDev />
              </button>
            ) : null}
          </>
        }
        fullscreen={fs.fullscreen}
        onToggleFullscreen={fs.toggleFullscreen}
        showStandTip={view !== "live" || arenaMode !== "competition"}
        nachkaufActive={
          view === "live" && arenaMode === "competition" && arenaNachkauf
        }
        onRequestSetup={() => {
          setView("live");
          setSetupRequestNonce((n) => n + 1);
        }}
      />

      <div className="app-frame-body">
        {/* Keep Arena mounted so Wettkampf-/Starter-Auswahl den Tab-Wechsel überlebt. */}
        <div
          className={view === "live" ? "app-view-pane" : "app-view-pane is-hidden"}
          aria-hidden={view !== "live"}
        >
          <LiveStandView
            trainingShooter={trainingShooter}
            onTrainingShooterChange={setTrainingShooter}
            playShot={sound.playShot}
            adminMode={isAdminModeEnabled}
            developerMode={isDeveloperModeEnabled}
            developerSheetOpen={isDeveloperSheetOpen}
            mouseAimEnabled={mouseAimEnabled}
            devShotInjectEpoch={devShotInjectEpoch}
            onOpenHistory={() => setView("history")}
            onArenaModeChange={setArenaMode}
            onNachkaufActiveChange={setArenaNachkauf}
            arenaVisible={view === "live"}
            competitionsEpoch={competitionsEpoch}
            competitionId={arenaCompetitionId ?? ""}
            onCompetitionIdChange={selectArenaCompetition}
            onBureauCompetitionIdChange={selectBureauCompetition}
            arenaHandoff={arenaHandoff}
            onArenaHandoffConsumed={() => setArenaHandoff(null)}
            setupRequestNonce={setupRequestNonce}
          />
        </div>
        {view === "bureau" ? (
          <BureauView
            adminMode={isAdminModeEnabled}
            selectedCompetitionId={bureauCompetitionId}
            onSelectedCompetitionIdChange={selectBureauCompetition}
            onActivateIntoArena={activateIntoArena}
            onClearArenaIfCompetition={clearArenaIfCompetition}
            onCompetitionsInvalidated={invalidateCompetitions}
            onOpenLive={(handoff) => {
              if (handoff) {
                setBureauCompetitionId(handoff.competitionId);
                setArenaHandoff(handoff);
              }
              setView("live");
            }}
          />
        ) : null}
        {view === "history" ? (
          <TrainingHistoryView defaultShooter={trainingShooter} />
        ) : null}
      </div>

      <SettingsSheet
        open={isSettingsSheetOpen}
        onClose={() => setSettingsSheetOpen(false)}
        adminAccessState={adminAccessState}
        isAdminModeEnabled={isAdminModeEnabled}
        onSearchDevice={() => {
          setSettingsSheetOpen(false);
          setView("live");
          setSetupRequestNonce((n) => n + 1);
        }}
        onDatabaseReplaced={() => {
          setSettingsSheetOpen(false);
          window.location.reload();
        }}
      />

      <DevPanel
        open={isDeveloperSheetOpen}
        onClose={() => setDeveloperSheetOpen(false)}
        onShotInjected={() => setDevShotInjectEpoch((n) => n + 1)}
        mouseAimEnabled={mouseAimEnabled}
        onMouseAimEnabledChange={setMouseAimEnabled}
        isAdminModeEnabled={isAdminModeEnabled}
        onAdminTestUnlockChange={setAdminUnlockedForTests}
        stackedSecondary={isSettingsSheetOpen && isDeveloperSheetOpen}
      />
    </div>
  );
}
