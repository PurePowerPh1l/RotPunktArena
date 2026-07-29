import { useCallback, useEffect, useState } from "react";
import * as api from "../api/commands";
import type { RecoverySessionInfo } from "@rotpunktarena/domain";
import { useAsyncAction } from "../hooks/useAsyncAction";

type Props = {
  sessions: RecoverySessionInfo[];
  onResolved: () => void;
};

function formatRelative(iso: string | null | undefined): string {
  if (!iso) return "unbekannt";
  try {
    const then = new Date(iso).getTime();
    const sec = Math.max(0, Math.round((Date.now() - then) / 1000));
    if (sec < 60) return `vor ${sec}s`;
    if (sec < 3600) return `vor ${Math.round(sec / 60)} Min.`;
    if (sec < 86400) return `vor ${Math.round(sec / 3600)} Std.`;
    return `vor ${Math.round(sec / 86400)} Tg.`;
  } catch {
    return iso;
  }
}

export function RecoveryGate({ sessions: initial, onResolved }: Props) {
  const [sessions, setSessions] = useState(initial);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const { busy, error, run } = useAsyncAction();

  useEffect(() => {
    setSessions(initial);
  }, [initial]);

  const refresh = useCallback(async () => {
    const next = await api.listRecoverySessions();
    setSessions(next);
    if (next.length === 0) onResolved();
  }, [onResolved]);

  const resume = async (id: string) => {
    await run(async () => {
      // Hardware sessions must resume on hardware — simulator only when the
      // RFCOMM/hardware link feature is not available (legacy field name).
      const live = await api.getLiveState().catch(() => null);
      const useSimulator = live ? live.serialFeature !== true : true;
      await api.resumeSession(id, useSimulator);
      const next = await api.listRecoverySessions();
      setSessions(next);
      if (next.length === 0) onResolved();
    });
  };

  const closeSafely = async (id: string) => {
    await run(async () => {
      await api.closeInterruptedSession(id);
      await refresh();
    });
  };

  const exportBundle = async () => {
    await run(async () => {
      const result = await api.exportDiagnostics();
      setExportPath(result.path);
    });
  };

  return (
    <div
      className="recovery-gate"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="recovery-title"
    >
      <div className="recovery-card">
        <p className="recovery-eyebrow">Autosave · Recovery</p>
        <h1 id="recovery-title">Unterbrochene Session</h1>
        <p className="recovery-lead">
          Der letzte Autosave-Marker zeigt eine Session ohne sauberen Abschluss.
          Fortsetzen lädt den gesicherten Stand; abschließen friert das Ergebnis ein.
        </p>

        {error ? <p className="banner-error">{error}</p> : null}
        {exportPath ? (
          <p className="recovery-export-ok">
            Diagnose-Export gespeichert: <code>{exportPath}</code>
          </p>
        ) : null}

        <ul className="recovery-list">
          {sessions.map((s) => (
            <li key={s.id} className="recovery-item">
              <div className="recovery-item-main">
                <strong>{s.shooterName}</strong>
                <span>
                  {s.competitionId ? "Wettkampf" : "Training"}
                  {" · "}
                  {s.shotCount} Schuss
                  {" · "}
                  letzter gesicherter Schuss #
                  {s.lastAutosaveSequence ?? "—"}
                  {" · "}
                  zuletzt gesichert {formatRelative(s.lastAutosaveAt)}
                </span>
              </div>
              <div className="recovery-item-actions">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void resume(s.id)}
                >
                  Fortsetzen
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={() => void closeSafely(s.id)}
                >
                  Sicher abschließen
                </button>
              </div>
            </li>
          ))}
        </ul>

        <div className="recovery-footer">
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={() => void exportBundle()}
          >
            Diagnose exportieren
          </button>
        </div>
      </div>
    </div>
  );
}
