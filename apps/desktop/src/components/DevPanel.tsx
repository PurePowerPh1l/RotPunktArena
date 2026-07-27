import { useCallback, useEffect, useState } from "react";
import type { DevDiagnostics } from "@reddot/domain";
import {
  assertCapability,
  getAppAccessSnapshot,
  hasCapability,
} from "../access";
import * as api from "../api/commands";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { SideSheetSection, SideSheetShell } from "./SideSheetShell";

type Props = {
  open: boolean;
  onClose: () => void;
  onShotInjected?: () => void;
  mouseAimEnabled: boolean;
  onMouseAimEnabledChange: (enabled: boolean) => void;
  /** Ephemeral admin test unlock — does not equal “developer sheet open”. */
  isAdminModeEnabled: boolean;
  onAdminTestUnlockChange: (on: boolean) => void;
  /** Shift left when Settings is also open. */
  stackedSecondary?: boolean;
};

export function DevPanel({
  open,
  onClose,
  onShotInjected,
  mouseAimEnabled,
  onMouseAimEnabledChange,
  isAdminModeEnabled,
  onAdminTestUnlockChange,
  stackedSecondary = false,
}: Props) {
  const [diag, setDiag] = useState<DevDiagnostics | null>(null);
  const { busy, error, setError, run } = useAsyncAction();
  const canDiagnostics = hasCapability(
    "developer:diagnostics",
    getAppAccessSnapshot(),
  );
  const canTestUnlock = hasCapability(
    "admin:test-unlock",
    getAppAccessSnapshot(),
  );

  const refresh = useCallback(async () => {
    assertCapability("developer:diagnostics", getAppAccessSnapshot());
    setError(null);
    setDiag(await api.devDiagnostics());
  }, [setError]);

  useEffect(() => {
    if (!open || !canDiagnostics) return;
    void refresh().catch((e) => setError(String(e)));
  }, [open, canDiagnostics, refresh, setError]);

  const inject = async () => {
    await run(async () => {
      assertCapability("developer:diagnostics", getAppAccessSnapshot());
      await api.devInjectTestShot();
      await refresh();
      onShotInjected?.();
    });
  };

  const exportBundle = async () => {
    await run(async () => {
      assertCapability("developer:diagnostics", getAppAccessSnapshot());
      const result = await api.exportDiagnostics();
      window.alert(`Diagnose-Export gespeichert:\n${result.path}`);
      await refresh();
    });
  };

  if (!open) return null;

  const schemaOk =
    diag?.hasCompetitionId &&
    diag?.hasEntryId &&
    (diag?.schemaVersion ?? 0) >= 4;

  return (
    <SideSheetShell
      title="Entwickler"
      ariaLabel="Entwickler"
      onClose={onClose}
      stackedSecondary={stackedSecondary}
    >
      {error ? <p className="banner-error">{error}</p> : null}

      <SideSheetSection label="Zugang (Test)">
        <label className="side-sheet-toggle">
          <input
            type="checkbox"
            checked={isAdminModeEnabled}
            disabled={!canTestUnlock}
            onChange={(e) => onAdminTestUnlockChange(e.target.checked)}
          />
          <span>
            Admin-Modus für Tests aktivieren
            <span className="field-hint">
              nur Development/Test · gilt bis App-Neustart · öffnet keinen
              anderen Modus
            </span>
          </span>
        </label>
      </SideSheetSection>

      <SideSheetSection label="Arena">
        <label className="side-sheet-toggle">
          <input
            type="checkbox"
            checked={mouseAimEnabled}
            onChange={(e) => onMouseAimEnabledChange(e.target.checked)}
          />
          <span>
            Mit Maus schießen
            <span className="field-hint">
              bleibt aktiv, wenn dieses Sheet geschlossen ist (braucht
              Entwickler-Fähigkeit, nicht Sheet-offen)
            </span>
          </span>
        </label>
        <div className="side-sheet-actions">
          <button
            type="button"
            className="secondary"
            disabled={busy || !canDiagnostics}
            onClick={() => void refresh().catch((e) => setError(String(e)))}
          >
            Status neu laden
          </button>
          <button
            type="button"
            disabled={busy || !canDiagnostics}
            onClick={() => void inject()}
          >
            Testschuss → DB
          </button>
          <button
            type="button"
            className="secondary"
            disabled={busy || !canDiagnostics}
            onClick={() => void exportBundle()}
          >
            Diagnose-ZIP
          </button>
        </div>
      </SideSheetSection>

      {diag ? (
        <SideSheetSection label="Diagnose">
          <dl className="side-sheet-stats">
            <dt>Schema</dt>
            <dd>
              v{diag.schemaVersion}{" "}
              {schemaOk ? (
                <span className="side-sheet-ok">OK</span>
              ) : (
                <span className="side-sheet-bad">Spalten fehlen?</span>
              )}
            </dd>
            <dt>competition_id</dt>
            <dd>{diag.hasCompetitionId ? "ja" : "nein"}</dd>
            <dt>Session-Schüsse</dt>
            <dd>
              DB {diag.sessionShots} · UI {diag.liveUiShots}
            </dd>
            <dt>Gesamt</dt>
            <dd>
              {diag.totalShots} Schüsse · {diag.totalFrames} Frames ·{" "}
              {diag.shotReceivedEvents} Events
            </dd>
            <dt>Session</dt>
            <dd className="side-sheet-mono">{diag.sessionId ?? "—"}</dd>
            <dt>DB-Pfad</dt>
            <dd className="side-sheet-mono">{diag.dbPath}</dd>
            {diag.uncleanSessions.length > 0 ? (
              <>
                <dt>Unclean</dt>
                <dd>{diag.uncleanSessions.length}</dd>
              </>
            ) : null}
          </dl>
        </SideSheetSection>
      ) : null}

      {diag && diag.recentShots.length > 0 ? (
        <SideSheetSection label="Letzte Schüsse (DB)">
          <ul className="side-sheet-list">
            {diag.recentShots.map((s) => (
              <li key={s.frameId}>
                #{s.shotIndex} · {s.score} · ({s.x},{s.y}) · seq{" "}
                {s.sessionSequence}
              </li>
            ))}
          </ul>
        </SideSheetSection>
      ) : null}

      <p className="hint">
        Sheet öffnen ≠ Modus aktiv. Testschuss nutzt denselben Arena-Ingest wie
        Hardware — zuerst Session starten.
      </p>
    </SideSheetShell>
  );
}
