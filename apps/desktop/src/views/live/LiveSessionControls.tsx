import type {
  Competition,
  CompetitionEntry,
  CompetitionTeam,
} from "@rotpunktarena/domain";
import {
  trainingEndlessResetConfirmMessage,
  trainingHistoryMinShotsHint,
  trainingResetConfirmMessage,
} from "@rotpunktarena/domain";
import type { ReactNode } from "react";
import { MagicButton } from "../../components/MagicButton";
import {
  ShooterAutocomplete,
  type ShooterValue,
} from "../../components/ShooterAutocomplete";
import { SlidingSeg } from "../../components/SlidingSeg";
import { confirmDialog } from "../../hooks/useAppDialog";
import {
  IconPlay,
  IconStop,
  IconTraining,
  IconTrophy,
} from "../../components/UiIcons";
import type { CompetitionCreateInput } from "../bureau/CompetitionCreateForm";
import { ControlsModeStack } from "./ControlsModeStack";
import { LiveCompetitionPicker } from "./LiveCompetitionPicker";
import {
  hardwareLiveStartBlocked,
  resolveLiveExplicitSimulator,
  resolveLiveHardwareStart,
  serialAvailabilityFromLive,
} from "./preferLiveSimulator";

type Props = {
  mode: "training" | "competition";
  onModeChange: (mode: "training" | "competition") => void;
  running: boolean;
  busy: boolean;
  sessionOpen: boolean;
  canAim: boolean;
  canStartCompetition: boolean;
  isTraining: boolean;
  endlessMode: boolean;
  shotCount: number;
  serialFeature: boolean | undefined;
  transport: string | undefined;
  autoFire: boolean | undefined;
  hasSession: boolean;
  trainingShooter: ShooterValue;
  onTrainingShooterChange: (next: ShooterValue) => void;
  competitions: Competition[];
  competitionId: string;
  onCompetitionIdChange: (id: string) => void;
  entries: CompetitionEntry[];
  entryId: string;
  onEntryIdChange: (id: string) => void;
  teams: CompetitionTeam[];
  teamId: string;
  onTeamIdChange: (id: string) => void;
  selectedComp: Competition | undefined;
  entryBlocked: boolean;
  /** Entry is done and Nachkauf is allowed — start opens a new series. */
  startNachkauf?: boolean;
  createBusy?: boolean;
  onCreateCompetition: (input: CompetitionCreateInput) => Promise<boolean>;
  onEnsureStarter: (shooter: ShooterValue) => Promise<boolean>;
  onEnsureTeam: (name: string) => Promise<boolean>;
  onStart: (useSimulator: boolean) => void;
  /** App-lifetime RFCOMM link is up — required for hardware training. */
  linkReady?: boolean;
  onStop: () => void;
  onStartNext: () => void;
  onReset: () => void;
  onFireOnce: () => void;
  onToggleAuto: () => void;
  adminMode?: boolean;
  developerMode?: boolean;
  /** Dev panel open — shows simulator fire helpers. */
  devOpen?: boolean;
  /** Training-only XP/Liga strip in the controls footer. */
  progressStrip?: ReactNode;
};

export function LiveSessionControls({
  mode,
  onModeChange,
  running,
  busy,
  sessionOpen,
  canAim,
  canStartCompetition,
  isTraining,
  endlessMode,
  shotCount,
  serialFeature,
  transport,
  autoFire,
  hasSession,
  trainingShooter,
  onTrainingShooterChange,
  competitions,
  competitionId,
  onCompetitionIdChange,
  entries,
  entryId,
  onEntryIdChange,
  teams,
  teamId,
  onTeamIdChange,
  selectedComp,
  entryBlocked,
  startNachkauf = false,
  createBusy = false,
  onCreateCompetition,
  onEnsureStarter,
  onEnsureTeam,
  onStart,
  linkReady = false,
  onStop,
  onStartNext,
  onReset,
  onFireOnce,
  onToggleAuto,
  adminMode = false,
  developerMode = false,
  devOpen = false,
  progressStrip = null,
}: Props) {
  const handleReset = () => {
    const msg = endlessMode
      ? trainingEndlessResetConfirmMessage()
      : trainingResetConfirmMessage();
    void (async () => {
      const ok = await confirmDialog({
        title: "Serie zurücksetzen?",
        body: msg,
        confirmLabel: "Zurücksetzen",
        danger: true,
        eyebrow: "Training",
      });
      if (!ok) return;
      onReset();
    })();
  };

  const competitionStartLabel = startNachkauf
    ? "Nachkauf starten"
    : "Wettkampf starten";

  const serial = serialAvailabilityFromLive(serialFeature);
  const canShowSimulatorButton =
    (adminMode || developerMode) && serial !== "unavailable";
  const hardwareBlocked = hardwareLiveStartBlocked({
    serial,
    linkReady,
  });
  /** Why the start row is greyed out — shown as tooltip on both buttons. */
  const startBlockedReason = !canStartCompetition
    ? mode === "competition"
      ? "Zuerst Starter aus der Startliste wählen"
      : "Zuerst Schütze eintragen oder aus der Liste wählen"
    : undefined;
  const hardwareTitle =
    startBlockedReason ??
    (serial === "unknown"
      ? "Live-Status wird geladen…"
      : serial === "available"
        ? linkReady
          ? mode === "competition"
            ? "Wettkampf starten — Schüsse am bestehenden Bluetooth-Link"
            : "Serie starten — Schüsse am bestehenden Bluetooth-Link"
          : "Zuerst RedDot einrichten (Setup-Hinweis) oder Simulator"
        : undefined);
  const simulatorTitle =
    startBlockedReason ?? "Nur Simulator — ohne Hardware";

  const handleHardwareStart = () => {
    const decision = resolveLiveHardwareStart({ serial, linkReady });
    if (decision.action === "blocked") return;
    onStart(decision.useSimulator);
  };

  const handleExplicitSimulator = () => {
    const decision = resolveLiveExplicitSimulator();
    onStart(decision.useSimulator);
  };

  return (
    <div className="controls-stack">
      {entryBlocked ? (
        <p className="controls-banner" role="status">
          Serie beendet
        </p>
      ) : startNachkauf ? (
        <p className="controls-banner controls-banner-nachkauf" role="status">
          Serie beendet — Nachkauf starten öffnet eine neue Serie.
        </p>
      ) : null}

      <footer className="controls" data-mode={mode}>
        <div className="controls-main">
          <SlidingSeg
            className="mode-row"
            ariaLabel="Modus"
            value={mode}
            disabled={running}
            onChange={onModeChange}
            options={[
              {
                value: "training",
                label: (
                  <span className="seg-label">
                    <IconTraining size={16} /> Training
                  </span>
                ),
              },
              {
                value: "competition",
                label: (
                  <span className="seg-label">
                    <IconTrophy size={16} /> Wettkampf
                  </span>
                ),
              },
            ]}
          />

          <ControlsModeStack
            mode={mode}
            training={
              <label className="field field-shooter">
                Schütze
                <ShooterAutocomplete
                  value={trainingShooter}
                  onChange={onTrainingShooterChange}
                  disabled={running}
                />
              </label>
            }
            competition={
              <LiveCompetitionPicker
                competitions={competitions}
                competitionId={competitionId}
                onCompetitionIdChange={onCompetitionIdChange}
                entries={entries}
                entryId={entryId}
                onEntryIdChange={onEntryIdChange}
                teams={teams}
                teamId={teamId}
                onTeamIdChange={onTeamIdChange}
                selectedComp={selectedComp}
                running={running}
                createBusy={createBusy}
                onCreateCompetition={onCreateCompetition}
                onEnsureStarter={onEnsureStarter}
                onEnsureTeam={onEnsureTeam}
              />
            }
          />
        </div>

        <div className="controls-actions">
          {!running && !sessionOpen ? (
            <>
              <MagicButton
                className={`nav-btn start-cta${startNachkauf ? " is-nachkauf" : ""}`}
                disabled={busy || !canStartCompetition || hardwareBlocked}
                title={hardwareTitle}
                onClick={handleHardwareStart}
              >
                <IconPlay />
                <span className="nav-btn-text">
                  {mode === "competition"
                    ? competitionStartLabel
                    : "Übung starten"}
                </span>
              </MagicButton>
              {canShowSimulatorButton ? (
                <button
                  type="button"
                  className="ghost nav-btn"
                  disabled={busy || !canStartCompetition}
                  title={simulatorTitle}
                  onClick={handleExplicitSimulator}
                >
                  <span className="nav-btn-text">Simulator</span>
                </button>
              ) : null}
            </>
          ) : (
            <>
              <button
                type="button"
                className="danger nav-btn"
                disabled={busy}
                onClick={onStop}
              >
                <IconStop />
                <span className="nav-btn-text">Stoppen</span>
              </button>
              {mode === "competition" ? (
                <button
                  type="button"
                  className="secondary"
                  disabled={busy || hardwareBlocked}
                  onClick={onStartNext}
                  title={
                    hardwareBlocked
                      ? hardwareTitle ??
                        "Nächster Schütze — Hardware-Start nicht bereit"
                      : "Aktuellen Schützen beenden und nächsten aus der Startliste starten"
                  }
                >
                  Nächster Schütze
                </button>
              ) : null}
            </>
          )}

          {isTraining && (sessionOpen || shotCount > 0) ? (
            <button
              type="button"
              className="ghost"
              disabled={busy || !hasSession}
              onClick={handleReset}
              title={
                endlessMode
                  ? "Endlos-Serie verwerfen und neu starten"
                  : `Serie beenden (${trainingHistoryMinShotsHint()}) und neue starten`
              }
            >
              Serie zurücksetzen
            </button>
          ) : null}

          {devOpen && sessionOpen ? (
            <>
              <MagicButton
                magnetic={false}
                disabled={!canAim}
                onClick={onFireOnce}
              >
                Schuss senden
              </MagicButton>
              {transport === "simulator" ? (
                <button
                  type="button"
                  className="secondary"
                  disabled={!canAim}
                  onClick={onToggleAuto}
                >
                  {autoFire ? "Auto-Schuss aus" : "Auto-Schuss an"}
                </button>
              ) : null}
            </>
          ) : null}
        </div>

        {progressStrip}
      </footer>
    </div>
  );
}
