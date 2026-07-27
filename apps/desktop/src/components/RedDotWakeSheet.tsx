import { useRef, useState } from "react";
import { MagicButton } from "./MagicButton";
import * as liveApi from "../api/live";

type Props = {
  open: boolean;
  targetName?: string | null;
  /** Owner status when idle/faulted/needsPairing — drives lead copy. */
  rfcommStatus?: string | null;
  reason?: string | null;
  onClose: () => void;
  onLinked: () => void;
};

/**
 * Small hint when Known target exists but link is idle (e.g. device asleep).
 * CTA = Nuclear repair — no silent AF_BTH from this sheet.
 */
export function RedDotWakeSheet({
  open,
  targetName,
  rfcommStatus,
  reason,
  onClose,
  onLinked,
}: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inflight = useRef(false);

  if (!open) return null;

  const name = targetName?.trim() || "RedDot";
  const faulted =
    rfcommStatus === "faulted" || rfcommStatus === "needsPairing";
  const title = faulted
    ? "Verbindung fehlgeschlagen"
    : "Nicht verbunden";
  const lead = faulted
    ? `${name} ist eingerichtet, aber der letzte Verbindungsversuch ist fehlgeschlagen.${
        reason?.trim() ? ` ${reason.trim()}` : ""
      } Einmal „Verbinden“ setzt die Verbindung neu auf.`
    : `${name} ist eingerichtet, aber gerade nicht verbunden — oft nach längerem Idle. Einmal verbinden stellt die Verbindung neu her.`;

  async function onConnect() {
    if (inflight.current) return;
    inflight.current = true;
    setBusy(true);
    setError(null);
    try {
      await liveApi.rfcommConnectReddot();
      // Parent closes without permanent dismiss (link-lost can show wake again).
      onLinked();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      inflight.current = false;
      setBusy(false);
    }
  }

  return (
    <div
      className="reddot-setup reddot-wake is-open"
      role="dialog"
      aria-modal="true"
      aria-labelledby="reddot-wake-title"
    >
      <button
        type="button"
        className="reddot-setup-backdrop"
        aria-label="Schließen"
        onClick={() => !busy && onClose()}
      />
      <div className="reddot-setup-panel reddot-wake-panel">
        <header className="reddot-setup-head">
          <h2 id="reddot-wake-title">{title}</h2>
          <button
            type="button"
            className="ghost"
            disabled={busy}
            onClick={onClose}
          >
            Später
          </button>
        </header>

        <p className="reddot-setup-lead">{lead}</p>

        {error ? (
          <p className="reddot-setup-error" role="alert">
            {error}
          </p>
        ) : null}

        <div className="reddot-setup-actions">
          <MagicButton
            className="nav-btn"
            disabled={busy}
            onClick={() => void onConnect()}
          >
            {busy ? "Verbinde…" : "Verbinden"}
          </MagicButton>
          <button
            type="button"
            className="secondary nav-btn"
            disabled={busy}
            onClick={onClose}
          >
            Später
          </button>
        </div>
      </div>
    </div>
  );
}
