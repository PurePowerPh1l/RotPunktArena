import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MagicButton } from "./MagicButton";
import * as liveApi from "../api/live";
import type { SetupCandidate } from "../api/live";

type Phase = "idle" | "searching" | "found" | "connecting" | "error" | "done";

type Props = {
  open: boolean;
  onClose: () => void;
  onLinked: () => void;
};

function candidateHint(c: SetupCandidate): string {
  if (c.isActive) return "Zuletzt verbunden";
  if (c.alreadyPaired) return "Bereits gekoppelt";
  return "Neu — falls Windows nach einer PIN fragt: 0000";
}

/**
 * Apple-style setup: proximity copy → scan → tap to connect → done.
 * Shows every RedDot in reach; one tap switches the active device.
 * Runtime autoconnect stays outside this sheet.
 */
export function RedDotSetupSheet({ open, onClose, onLinked }: Props) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [candidates, setCandidates] = useState<SetupCandidate[]>([]);
  const [selected, setSelected] = useState<SetupCandidate | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) {
      setPhase("idle");
      setCandidates([]);
      setSelected(null);
      setError(null);
      setBusy(false);
      return;
    }
    // Auto-start scan when sheet opens.
    void runScan();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  async function runScan() {
    setBusy(true);
    setError(null);
    setCandidates([]);
    setSelected(null);
    setPhase("searching");
    try {
      const found = await liveApi.rfcommSetupScan();
      setCandidates(found);
      setPhase("found");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase("error");
    } finally {
      setBusy(false);
    }
  }

  const [connectHint, setConnectHint] = useState("Gerät wird vorbereitet…");

  async function runConnect(candidate: SetupCandidate) {
    setSelected(candidate);
    setBusy(true);
    setError(null);
    setPhase("connecting");
    setConnectHint("Gerät wird vorbereitet…");
    await new Promise<void>((r) => requestAnimationFrame(() => r()));
    const poll = window.setInterval(() => {
      void invoke<{ reason?: string }>("rfcomm_status").then((s) => {
        if (s.reason) setConnectHint(s.reason);
      });
    }, 400);
    try {
      await liveApi.rfcommSetupConnect(
        candidate.btAddrHex,
        candidate.displayName,
      );
      setPhase("done");
      onLinked();
      window.setTimeout(() => onClose(), 700);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase("error");
    } finally {
      window.clearInterval(poll);
      setBusy(false);
    }
  }

  if (!open) return null;

  const single = candidates.length === 1 ? candidates[0] : null;

  return (
    <div className="reddot-setup is-open" role="dialog" aria-modal="true" aria-labelledby="reddot-setup-title">
      <button
        type="button"
        className="reddot-setup-backdrop"
        aria-label="Schließen"
        onClick={onClose}
      />
      <div className="reddot-setup-panel">
        <header className="reddot-setup-head">
          <h2 id="reddot-setup-title">RedDot einrichten</h2>
          {/* Always dismissible — scan/connect continue in the background.
              Blocking dismiss also blocks the simulator start behind the sheet. */}
          <button type="button" className="ghost" onClick={onClose}>
            Später
          </button>
        </header>

        <p className="reddot-setup-lead">
          Schalte dein RedDot-Ziel ein und halte es nah am PC.
        </p>

        {phase === "searching" ? (
          <p className="reddot-setup-status" aria-live="polite">
            Suche…
          </p>
        ) : null}

        {phase === "found" && single ? (
          <div className="reddot-setup-found">
            <p className="reddot-setup-status" aria-live="polite">
              Gefunden: <strong>{single.displayName}</strong>
            </p>
            <p className="reddot-setup-hint muted">
              {single.alreadyPaired
                ? "Bereits gekoppelt — Verbindung wird aufgebaut."
                : "Falls Windows nach einer PIN fragt: 0000"}
            </p>
          </div>
        ) : null}

        {phase === "found" && candidates.length > 1 ? (
          <div className="reddot-setup-found">
            <p className="reddot-setup-status" aria-live="polite">
              {candidates.length} RedDots gefunden — tippe zum Verbinden:
            </p>
            <ul className="reddot-setup-list">
              {candidates.map((c) => (
                <li key={c.btAddrHex}>
                  <button
                    type="button"
                    className="reddot-setup-device"
                    disabled={busy}
                    onClick={() => void runConnect(c)}
                  >
                    <span className="reddot-setup-device-name">
                      {c.displayName}
                    </span>
                    <span className="reddot-setup-device-hint muted">
                      {candidateHint(c)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        {phase === "connecting" ? (
          <div className="reddot-setup-found">
            <p className="reddot-setup-status is-busy" aria-live="polite">
              {connectHint}
            </p>
            <p className="reddot-setup-hint muted">
              Frische Verbindung zum RedDot
              {selected ? ` · ${selected.displayName}` : ""}
            </p>
          </div>
        ) : null}

        {phase === "done" ? (
          <p className="reddot-setup-status is-ok" aria-live="polite">
            Verbunden
          </p>
        ) : null}

        {phase === "error" && error ? (
          <p className="reddot-setup-error" role="alert">
            Verbindung neu aufsetzen fehlgeschlagen — {error}
          </p>
        ) : null}

        <div className="reddot-setup-actions">
          {phase === "found" && single ? (
            <MagicButton
              className="nav-btn"
              disabled={busy}
              onClick={() => void runConnect(single)}
            >
              Verbinden
            </MagicButton>
          ) : null}
          {phase === "connecting" ? (
            <button type="button" className="secondary nav-btn is-busy" disabled>
              Verbinde…
            </button>
          ) : null}
          {phase === "error" && selected ? (
            <MagicButton
              className="nav-btn"
              disabled={busy}
              onClick={() => void runConnect(selected)}
            >
              Erneut versuchen
            </MagicButton>
          ) : null}
          {phase === "error" || phase === "idle" ? (
            <MagicButton
              className="nav-btn"
              disabled={busy}
              onClick={() => void runScan()}
            >
              Erneut suchen
            </MagicButton>
          ) : null}
          {phase === "searching" ? (
            <button type="button" className="secondary nav-btn" disabled>
              Suche…
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
