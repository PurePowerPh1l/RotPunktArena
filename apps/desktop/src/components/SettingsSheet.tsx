import { useEffect, useMemo, useState } from "react";
import type { UiPrefs } from "@rotpunktarena/domain";
import {
  adminAccessStore,
  assertCapability,
  getAppAccessSnapshot,
  requireAdminAuth,
  type AdminAccessState,
} from "../access";
import * as api from "../api/commands";
import { changeAdminPassword } from "../api/adminAuth";
import { exportTrainingGoals } from "../training/goals";
import type { BackupHealth, DbBackupInfo } from "../api/admin";
import {
  alertDialog,
  confirmDialog,
} from "../hooks/useAppDialog";
import { useAppUpdateContext } from "../hooks/AppUpdateProvider";
import type { UiPrefsStatus } from "../hooks/useUiPrefs";
import type { AppView } from "./appNav";
import { SearchSelect } from "./SearchSelect";
import { UpdateSettings } from "./settings/UpdateSettings";
import { ConnectionSettings } from "./settings/ConnectionSettings";
import { GeneralSettings, AppearanceSettings, ArenaSettings } from "./settings/SettingsPreferences";
import { SideSheetShell } from "./SideSheetShell";
import {
  SettingsHint,
  SettingsInfoRow,
  SettingsLockedCard,
  SettingsSection,
} from "./settings/SettingsParts";

type SettingsSectionId =
  | "allgemein"
  | "app"
  | "darstellung"
  | "arena"
  | "verbindung"
  | "backups"
  | "admin";

type Props = {
  open: boolean;
  onClose: () => void;
  adminAccessState: AdminAccessState;
  isAdminModeEnabled: boolean;
  /** After restore/reset — reload app data. */
  onDatabaseReplaced?: () => void;
  /** Opens device discovery / first-setup sheet (Gerät suchen). */
  onSearchDevice?: () => void;
  /** General prefs — owned by App via useUiPrefs (no direct invoke here). */
  uiPrefs: UiPrefs;
  uiPrefsStatus: UiPrefsStatus;
  uiPrefsError: string | null;
  onUpdateUiPrefs: (patch: Partial<UiPrefs>) => void;
  onRetryUiPrefs?: () => void;
  /** Current main nav view — used when enabling rememberLastView. */
  currentView: AppView;
};

function formatBackupWhen(backup: DbBackupInfo | undefined): string {
  if (!backup) return "Noch kein Backup erstellt";
  if (backup.modifiedAt) {
    try {
      const d = new Date(backup.modifiedAt);
      if (!Number.isNaN(d.getTime())) {
        return d.toLocaleString("de-DE", {
          dateStyle: "medium",
          timeStyle: "short",
        });
      }
    } catch {
      /* fall through */
    }
  }
  return backup.name;
}

export function SettingsSheet({
  open,
  onClose,
  adminAccessState,
  isAdminModeEnabled,
  onDatabaseReplaced,
  onSearchDevice,
  uiPrefs,
  uiPrefsStatus,
  uiPrefsError,
  onUpdateUiPrefs,
  onRetryUiPrefs,
  currentView,
}: Props) {
  const appUpdate = useAppUpdateContext();
  const [health, setHealth] = useState<BackupHealth | null>(null);
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmedPassword, setConfirmedPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [backups, setBackups] = useState<DbBackupInfo[]>([]);
  const [selectedBackup, setSelectedBackup] = useState("");
  const [lastCreatedPath, setLastCreatedPath] = useState<string | null>(null);
  /** Single-open accordion; Verbindung is the default. */
  const [openSection, setOpenSection] = useState<SettingsSectionId | null>(
    "verbindung",
  );

  const toggleSection = (id: SettingsSectionId) => {
    setOpenSection((prev) => (prev === id ? null : id));
  };

  const canRestore = isAdminModeEnabled;
  const canReset = isAdminModeEnabled;

  const latestBackup = useMemo(() => backups[0], [backups]);

  const reloadBackups = async () => {
    const [list, status] = await Promise.all([api.listDbBackups(), api.getBackupHealth()]);
    setHealth(status);
    setBackups(list);
    if (selectedBackup && !list.some((b) => b.name === selectedBackup)) {
      setSelectedBackup("");
    }
  };

  useEffect(() => {
    if (!open) return;
    setOpenSection("verbindung");
    setError(null);
    void reloadBackups().catch((e) => setError(String(e)));
    void appUpdate.refreshVersion();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- Verbindung default + reload when sheet opens
  }, [open]);

  useEffect(() => {
    if (!open) { setCurrentPassword(""); setNewPassword(""); setConfirmedPassword(""); return; }
    const timer = window.setInterval(() => void api.getBackupHealth().then(setHealth).catch(e => setError(String(e))), 5000);
    return () => window.clearInterval(timer);
  }, [open]);

  if (!open) return null;

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const onBackup = () =>
    void run(async () => {
      assertCapability("backup:create", getAppAccessSnapshot());
      const info = await api.createDbBackup();
      setLastCreatedPath(info.path);
      await reloadBackups();
      setSelectedBackup(info.name);
    });

  const onRestore = () =>
    void run(async () => {
      if (!(await requireAdminAuth())) return;
      assertCapability("backup:restore", getAppAccessSnapshot());
      if (!selectedBackup) {
        setError("Backup wählen.");
        return;
      }
      const ok = await confirmDialog({
        title: "Backup wiederherstellen?",
        body: `Backup „${selectedBackup}“ wiederherstellen?\n\nAktuelle Daten werden ersetzt. Laufende Arena-Sessions müssen beendet sein.`,
        confirmLabel: "Wiederherstellen",
        danger: true,
        eyebrow: "Backup",
      });
      if (!ok) return;
      await api.restoreDbBackup(selectedBackup);
      onDatabaseReplaced?.();
      await alertDialog({
        title: "Backup wiederhergestellt",
        body: "Die Datenbank wurde aus dem gewählten Backup geladen.",
        eyebrow: "Backup",
      });
    });

  const onReset = () =>
    void run(async () => {
      if (!(await requireAdminAuth())) return;
      assertCapability("admin:reset", getAppAccessSnapshot());
      const ok = await confirmDialog({
        title: "Alle Daten zurücksetzen?",
        body: "Wirklich ALLE Daten zurücksetzen?\n\nSchützen, Wettkämpfe, Training, Teams — alles weg. Besser vorher ein Backup machen.",
        confirmLabel: "Zurücksetzen",
        danger: true,
        eyebrow: "Achtung",
      });
      if (!ok) return;
      const ok2 = await confirmDialog({
        title: "Letzte Warnung",
        body: "Datenbank wirklich leeren?",
        confirmLabel: "Endgültig leeren",
        danger: true,
        eyebrow: "Achtung",
      });
      if (!ok2) return;
      await api.resetAllDatabase();
      onDatabaseReplaced?.();
      await alertDialog({
        title: "Datenbank zurückgesetzt",
        body: "Alle lokalen Daten wurden gelöscht.",
        eyebrow: "Admin",
      });
    });

  const onAdminUnlockCta = () => {
    void requireAdminAuth();
  };

  const storagePath =
    lastCreatedPath ??
    latestBackup?.path ??
    "Wird beim ersten Backup angezeigt";

  return (
    <SideSheetShell
      title="Einstellungen"
      ariaLabel="Einstellungen"
      onClose={onClose}
      className="settings-sheet"
    >
      {error ? <p className="banner-error">{error}</p> : null}

      <GeneralSettings open={openSection === "allgemein"}
        onOpenChange={() => toggleSection("allgemein")}
        uiPrefs={uiPrefs} uiPrefsStatus={uiPrefsStatus} uiPrefsError={uiPrefsError}
        onRetryUiPrefs={onRetryUiPrefs} onUpdateUiPrefs={onUpdateUiPrefs}
        currentView={currentView}
      />

      <UpdateSettings open={openSection === "app"}
        onOpenChange={() => toggleSection("app")} appUpdate={appUpdate} />

      <AppearanceSettings open={openSection === "darstellung"}
        onOpenChange={() => toggleSection("darstellung")}
        uiPrefs={uiPrefs} uiPrefsStatus={uiPrefsStatus}
        onUpdateUiPrefs={onUpdateUiPrefs}
      />

      <ArenaSettings open={openSection === "arena"}
        onOpenChange={() => toggleSection("arena")}
        uiPrefs={uiPrefs} uiPrefsStatus={uiPrefsStatus}
        onUpdateUiPrefs={onUpdateUiPrefs}
      />

      <ConnectionSettings open={openSection === "verbindung"}
        onOpenChange={() => toggleSection("verbindung")} onSearchDevice={onSearchDevice} />

      <SettingsSection
        title="Daten & Backups"
        description="Sichere deine Daten und exportiere den aktuellen Stand."
        open={openSection === "backups"}
        onOpenChange={() => toggleSection("backups")}
      >
        <div className="settings-info-block">
          <SettingsInfoRow
            label="Letztes Backup"
            value={formatBackupWhen(latestBackup)}
          />
          <SettingsInfoRow
            label="Speicherort"
            value={storagePath}
            variant="path"
          />
        </div>
        {health ? <div className="settings-info-block">
          <SettingsInfoRow label="Automatische Sicherung" value={health.snapshot.lastCompletedAt ? new Date(health.snapshot.lastCompletedAt).toLocaleString("de-DE") : "Noch kein Snapshot in dieser App-Sitzung"} />
          <SettingsInfoRow label="Daten und Sicherungen" value={`${(health.storageBytes / 1048576).toFixed(1)} MiB`} />
          <SettingsInfoRow label="Übersprungene Sicherungen" value={String(health.snapshot.queueDrops)} />
          {health.snapshot.lastError ? <p className="banner-error">{health.snapshot.lastError}</p> : null}
        </div> : null}
        <div className="settings-backup-actions">
          <button
            type="button"
            className="settings-action-primary"
            disabled={busy}
            onClick={onBackup}
          >
            {busy ? "…" : "Backup erstellen"}
          </button>
        </div>
        <div className="side-sheet-actions">
          <button type="button" disabled={busy} onClick={() => void run(async () => { await api.retrySnapshot(); await reloadBackups(); })}>Automatische Sicherung erneut anfordern</button>
          <button type="button" disabled={busy} onClick={() => void run(async () => {
            if (!(await requireAdminAuth())) return;
            if (!(await confirmDialog({title:"Persönliches Gesamtbackup erstellen?",body:"Enthält alle Personen, Wettkämpfe, Trainingsdaten, Admin-Passworthash, Gerätegedächtnis und Trainingsziele. Nur an vertrauenswürdige Personen weitergeben.",confirmLabel:"Gesamtbackup erstellen"}))) return;
            const result = await api.exportPersonalBackup(exportTrainingGoals());
            setLastCreatedPath(result.path); await reloadBackups();
          })}>Gesamtbackup (DB, Geräte, Ziele)</button>
        </div>
        <SettingsHint>
          Ein Backup enthält die aktuellen Arena- und Sitzungsdaten.
        </SettingsHint>
      </SettingsSection>

      <SettingsSection
        title="Admin"
        description="Geschützte Verwaltungsaktionen."
        danger={isAdminModeEnabled}
        open={openSection === "admin"}
        onOpenChange={() => toggleSection("admin")}
      >
        <SettingsInfoRow
          label="Status"
          value={
            isAdminModeEnabled
              ? "Entsperrt"
              : adminAccessState === "locked"
                ? "Gesperrt"
                : "Noch nicht gesetzt"
          }
          statusTone={
            isAdminModeEnabled
              ? "ok"
              : adminAccessState === "locked"
                ? "locked"
                : "neutral"
          }
        />
        {!isAdminModeEnabled ? (
          <div className="side-sheet-actions">
            <button type="button" className="secondary" onClick={onAdminUnlockCta}>
              {adminAccessState === "locked"
                ? "Admin entsperren"
                : "Admin-Passwort setzen"}
            </button>
          </div>
        ) : (
          <div className="side-sheet-actions">
            <button
              type="button"
              className="ghost"
              onClick={() => adminAccessStore.lock()}
            >
              Admin sperren
            </button>
          </div>
        )}
        {isAdminModeEnabled ? (
          <>
            <div className="settings-info-block">
              <p>Admin-Passwort ändern — danach erneut entsperren.</p>
              <label className="field">Aktuelles Passwort<input type="password" autoComplete="current-password" value={currentPassword} onChange={e => setCurrentPassword(e.target.value)} disabled={busy} /></label>
              <label className="field">Neues Passwort<input type="password" autoComplete="new-password" value={newPassword} onChange={e => setNewPassword(e.target.value)} disabled={busy} /></label>
              <label className="field">Neues Passwort bestätigen<input type="password" autoComplete="new-password" value={confirmedPassword} onChange={e => setConfirmedPassword(e.target.value)} disabled={busy} /></label>
              <button type="button" disabled={busy || !currentPassword || newPassword.length < 8 || newPassword !== confirmedPassword} onClick={() => void run(async () => {
                await changeAdminPassword(currentPassword,newPassword);
                setCurrentPassword("");setNewPassword("");setConfirmedPassword("");adminAccessStore.lock();
                await alertDialog({title:"Passwort geändert",body:"Die Admin-Sitzung wurde gesperrt. Bitte mit dem neuen Passwort entsperren."});
              })}>Passwort ändern</button>
            </div>
            <label className="field">
              Backup wiederherstellen
              <SearchSelect
                value={selectedBackup}
                options={backups.map((b) => ({
                  id: b.name,
                  label: b.name,
                }))}
                onChange={setSelectedBackup}
                disabled={busy || !canRestore}
                placeholder={
                  backups.length ? "Backup wählen…" : "Kein Backup vorhanden"
                }
                allowClear
              />
            </label>
            <div className="side-sheet-actions">
              <button
                type="button"
                className="secondary"
                disabled={busy || !canRestore || !selectedBackup}
                onClick={onRestore}
              >
                Wiederherstellen
              </button>
            </div>
            <div className="settings-danger-block">
              <p className="settings-section-title settings-danger-label">
                Gefahrenzone
              </p>
              <SettingsHint>
                Löscht alle Schützen, Wettkämpfe und Trainingsdaten.
              </SettingsHint>
              <div className="side-sheet-actions">
                <button
                  type="button"
                  className="danger"
                  disabled={busy || !canReset}
                  onClick={onReset}
                >
                  Alle Daten zurücksetzen
                </button>
              </div>
            </div>
          </>
        ) : (
          <SettingsLockedCard
            statusLabel={
              adminAccessState === "locked" ? "Gesperrt" : "Noch nicht gesetzt"
            }
          >
            <p>
              Wiederherstellen und Zurücksetzen sind erst nach Admin-Entsperren
              verfügbar.
            </p>
          </SettingsLockedCard>
        )}
      </SettingsSection>
    </SideSheetShell>
  );
}
