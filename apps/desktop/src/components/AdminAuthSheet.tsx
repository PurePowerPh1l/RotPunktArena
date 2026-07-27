import { useEffect, useState } from "react";
import { adminAccessStore } from "../access/adminAccessStore.ts";
import {
  completeAdminAuth,
  registerAdminAuthUi,
  type AdminAuthMode,
} from "../access/requireAdminAuth.ts";
import { SideSheetShell, SideSheetSection } from "./SideSheetShell";

type Props = {
  stackedSecondary?: boolean;
};

/**
 * Single Admin-Passwort sheet — opened only via requireAdminAuth().
 */
export function AdminAuthSheet({ stackedSecondary = false }: Props) {
  const [mode, setMode] = useState<AdminAuthMode | null>(null);
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    registerAdminAuthUi((next) => {
      setPassword("");
      setConfirm("");
      setError(null);
      setMode(next);
    });
    return () => registerAdminAuthUi(null);
  }, []);

  if (!mode) return null;

  const title =
    mode === "setup" ? "Admin-Passwort setzen" : "Admin entsperren";

  const close = (ok: boolean) => {
    setMode(null);
    setPassword("");
    setConfirm("");
    setError(null);
    setBusy(false);
    completeAdminAuth(ok);
  };

  const onSubmit = () => {
    void (async () => {
      setBusy(true);
      setError(null);
      try {
        if (mode === "setup") {
          if (password !== confirm) {
            setError("Passwörter stimmen nicht überein.");
            return;
          }
          await adminAccessStore.setupPassword(password);
          close(true);
          return;
        }
        const ok = await adminAccessStore.unlock(password);
        if (!ok) {
          setError("Admin-Passwort ist falsch.");
          return;
        }
        close(true);
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    })();
  };

  return (
    <SideSheetShell
      title={title}
      ariaLabel={title}
      onClose={() => close(false)}
      stackedSecondary={stackedSecondary}
      className="admin-auth-sheet"
    >
      <SideSheetSection label="Admin-Passwort">
        <p className="settings-hint">
          {mode === "setup"
            ? "Lege ein Admin-Passwort fest, um geschützte Aktionen freizuschalten."
            : "Gib das Admin-Passwort ein, um fortzufahren."}
        </p>
        <label className="field">
          Admin-Passwort
          <input
            type="password"
            autoComplete="new-password"
            value={password}
            disabled={busy}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") onSubmit();
            }}
          />
        </label>
        {mode === "setup" ? (
          <label className="field">
            Admin-Passwort bestätigen
            <input
              type="password"
              autoComplete="new-password"
              value={confirm}
              disabled={busy}
              onChange={(e) => setConfirm(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") onSubmit();
              }}
            />
          </label>
        ) : null}
        {error ? <p className="settings-hint">{error}</p> : null}
        <div className="side-sheet-actions">
          <button
            type="button"
            className="ghost"
            disabled={busy}
            onClick={() => close(false)}
          >
            Abbrechen
          </button>
          <button
            type="button"
            disabled={busy || !password}
            onClick={onSubmit}
          >
            {mode === "setup" ? "Setzen und freischalten" : "Entsperren"}
          </button>
        </div>
      </SideSheetSection>
    </SideSheetShell>
  );
}
