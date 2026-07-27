import { useLiveLinkStatus } from "../hooks/useLiveLinkStatus";
import * as liveApi from "../api/live";
import { useEffect, useRef, useState } from "react";

type Props = {
  /** First-setup: badge click opens RedDotSetupSheet (Live). */
  onRequestSetup?: () => void;
};

type BadgeTone = "ok" | "idle" | "busy" | "setup" | "fault";

function toneClass(tone: BadgeTone): string {
  switch (tone) {
    case "ok":
      return "live-link-connected";
    case "busy":
      return "live-link-searching";
    case "idle":
      return "live-link-idle";
    case "setup":
      return "live-link-setup";
    case "fault":
      return "live-link-fault";
  }
}

/** Compact RFCOMM link indicator — app-lifetime, independent of training session. */
export function LiveLinkBadge({ onRequestSetup }: Props) {
  const {
    rfcommStatus,
    targetName,
    reason,
    needsSetup,
    linked,
    hasTarget,
    rfcommFeature,
    refresh,
  } = useLiveLinkStatus();
  const [busy, setBusy] = useState(false);
  const [lastError, setLastError] = useState<string | null>(null);
  const [diagOpen, setDiagOpen] = useState(false);
  const [diagRows, setDiagRows] = useState<liveApi.RfcommDiagEvent[]>([]);
  const inflight = useRef(false);

  const connecting =
    rfcommStatus === "connecting" ||
    rfcommStatus === "discovering" ||
    rfcommStatus === "reconnecting" ||
    busy;
  const faulted =
    rfcommStatus === "faulted" ||
    rfcommStatus === "needsPairing" ||
    Boolean(lastError);

  useEffect(() => {
    if (linked) setLastError(null);
  }, [linked]);

  useEffect(() => {
    if (!diagOpen) return;
    let cancelled = false;
    void (async () => {
      try {
        const rows = await liveApi.rfcommDiagTail(12);
        if (!cancelled) setDiagRows(rows);
      } catch {
        if (!cancelled) setDiagRows([]);
      }
    })();
    const id = window.setInterval(() => {
      void liveApi.rfcommDiagTail(12).then((rows) => {
        if (!cancelled) setDiagRows(rows);
      });
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [diagOpen]);

  let tone: BadgeTone = "idle";
  let label = "Nicht verbunden";
  let actionHint = "Verbinden";

  if (!rfcommFeature) {
    label = "RFCOMM aus";
    actionHint = "";
  } else if (needsSetup || (!hasTarget && !linked)) {
    tone = "setup";
    label = "Gerät einrichten";
    actionHint = "";
  } else if (linked) {
    tone = "ok";
    label = "Verbunden";
    actionHint = targetName ?? "";
  } else if (connecting) {
    tone = "busy";
    label = targetName ? `Verbinde mit ${targetName}…` : "Verbinde…";
    actionHint = "Abbrechen";
  } else if (faulted) {
    tone = "fault";
    label = "Verbindung fehlgeschlagen";
    actionHint = needsSetup ? "Gerät auswählen" : "Verbinden";
  } else {
    tone = "idle";
    label = "Nicht verbunden";
    actionHint = "Verbinden";
  }

  const canOpenSetup = Boolean(needsSetup && onRequestSetup && rfcommFeature);
  const canRepair = Boolean(
    rfcommFeature && !needsSetup && hasTarget && !linked && !connecting,
  );
  const canCancel = Boolean(rfcommFeature && connecting);

  const detail = [label, targetName, reason, lastError]
    .filter(Boolean)
    .join(" · ");

  async function onRepair() {
    if (inflight.current || linked || needsSetup) return;
    inflight.current = true;
    setBusy(true);
    setLastError(null);
    try {
      await liveApi.rfcommConnectReddot();
      await refresh();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setLastError(msg);
      await refresh();
    } finally {
      inflight.current = false;
      setBusy(false);
    }
  }

  async function onCancel() {
    try {
      await liveApi.rfcommCancelConnect();
      await refresh();
    } catch (e: unknown) {
      console.error(e);
    } finally {
      inflight.current = false;
      setBusy(false);
    }
  }

  const primaryClick = canOpenSetup
    ? () => onRequestSetup?.()
    : canCancel
      ? () => void onCancel()
      : canRepair
        ? () => void onRepair()
        : undefined;

  // Only force-expand while connecting so „Abbrechen“ sichtbar bleibt.
  // Idle/setup/fault: kompakter Punkt, Status per Hover (Slot + Badge).
  const expanded = connecting;

  return (
    <div className="live-link-wrap">
      <div
        className={`live-link ${toneClass(tone)}${canOpenSetup || canRepair ? " is-action" : ""}${canCancel ? " is-cancel" : ""}${expanded ? " is-expanded" : ""}`}
        title={
          linked
            ? `${label}${targetName ? ` · ${targetName}` : ""}`
            : `${detail} — Rechtsklick: Diagnose`
        }
        role={primaryClick ? "button" : "status"}
        aria-label={detail}
        tabIndex={primaryClick ? 0 : undefined}
        onClick={primaryClick ? () => primaryClick() : undefined}
        onContextMenu={(e) => {
          e.preventDefault();
          setDiagOpen((v) => !v);
        }}
        onKeyDown={
          primaryClick
            ? (e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  primaryClick();
                }
              }
            : undefined
        }
      >
        <span className="live-link-dot" aria-hidden />
        <span className="live-link-text">
          <span className="live-link-label">{label}</span>
          {linked && targetName ? (
            <span className="live-link-port">{targetName}</span>
          ) : null}
          {!linked && actionHint && tone !== "busy" ? (
            <span className="live-link-port">{actionHint}</span>
          ) : null}
          {canCancel ? (
            <span className="live-link-port live-link-cancel-hint">Abbrechen</span>
          ) : null}
        </span>
      </div>

      {diagOpen ? (
        <div className="live-link-diag" role="dialog" aria-label="RFCOMM-Diagnose">
          <header className="live-link-diag-head">
            <strong>Diagnose</strong>
            <button
              type="button"
              className="ghost"
              onClick={() => setDiagOpen(false)}
            >
              Schließen
            </button>
          </header>
          <p className="live-link-diag-lead muted">
            Letzte Ereignisse (Adresse anonymisiert). Rechtsklick auf Badge öffnet/schließt.
          </p>
          <ul className="live-link-diag-list">
            {diagRows.length === 0 ? (
              <li className="muted">Noch keine Ereignisse</li>
            ) : (
              [...diagRows].reverse().map((row, i) => (
                <li key={`${row.ts}-${i}`}>
                  <span className="live-link-diag-ts">
                    {row.ts.replace("T", " ").replace(/\.\d+Z$/, "Z")}
                  </span>
                  <span className="live-link-diag-ev">{row.event}</span>
                  {row.generation != null ? (
                    <span className="muted">g{row.generation}</span>
                  ) : null}
                  {row.addr ? <span className="muted">{row.addr}</span> : null}
                  {row.winsock != null ? (
                    <span className="muted">
                      {row.winsockName ?? "WSA"} {row.winsock}
                    </span>
                  ) : null}
                  <span className="live-link-diag-reason">{row.reason}</span>
                </li>
              ))
            )}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
