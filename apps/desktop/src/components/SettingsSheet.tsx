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
import * as liveApi from "../api/live";
import type { DbBackupInfo } from "../api/admin";
import { useLiveLinkStatus } from "../hooks/useLiveLinkStatus";
import {
  alertDialog,
  confirmDialog,
} from "../hooks/useAppDialog";
import { useAppUpdateContext } from "../hooks/AppUpdateProvider";
import type { AppUpdateStatus } from "../hooks/useAppUpdate";
import type { UiPrefsStatus } from "../hooks/useUiPrefs";
import type { AppView } from "./appNav";
import { SearchSelect } from "./SearchSelect";
import { SideSheetShell } from "./SideSheetShell";
import {
  SettingsChoice,
  SettingsHint,
  SettingsInfoRow,
  SettingsLockedCard,
  SettingsSection,
  SettingsToggle,
  type SettingsStatusTone,
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

function connectionStatusLabel(opts: {
  linked: boolean;
  hasTarget: boolean;
  connecting: boolean;
}): string {
  if (!opts.hasTarget) return "Kein Gerät ausgewählt";
  if (opts.linked) return "Verbunden";
  if (opts.connecting) return "Verbindet…";
  return "Nicht verbunden";
}

function connectionStatusTone(opts: {
  linked: boolean;
  hasTarget: boolean;
  connecting: boolean;
}): SettingsStatusTone {
  if (!opts.hasTarget) return "neutral";
  if (opts.linked) return "ok";
  if (opts.connecting) return "progress";
  return "idle";
}

function updateStatusLabel(status: AppUpdateStatus): string {
  switch (status.kind) {
    case "idle":
      return "Noch nicht geprüft";
    case "checking":
      return "Suche…";
    case "upToDate":
      return "Aktuell";
    case "available":
      return `Update ${status.update.version} verfügbar`;
    case "downloading": {
      const { downloaded, contentLength } = status.progress;
      if (contentLength && contentLength > 0) {
        const pct = Math.min(100, Math.round((downloaded / contentLength) * 100));
        return `Lade herunter… ${pct}%`;
      }
      return "Lade herunter…";
    }
    case "installing":
      return "Installiere…";
    case "readyToRelaunch":
      return `Installiert — Neustart nötig (${status.update.version})`;
    case "needsManualRestart":
      return `Neustart manuell nötig (${status.update.version})`;
    case "error":
      return "Fehler";
    case "devOnly":
      return "Nur in installierter App";
  }
}

function updateStatusTone(status: AppUpdateStatus): SettingsStatusTone {
  switch (status.kind) {
    case "idle":
      return "neutral";
    case "checking":
    case "downloading":
    case "installing":
      return "progress";
    case "upToDate":
      return "ok";
    case "available":
      return "idle";
    case "readyToRelaunch":
    case "needsManualRestart":
      return "idle";
    case "error":
      return "idle";
    case "devOnly":
      return "neutral";
  }
}

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
  const link = useLiveLinkStatus();
  const appUpdate = useAppUpdateContext();
  const [busy, setBusy] = useState(false);
  const [linkBusy, setLinkBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const prefsReady = uiPrefsStatus === "ready" || uiPrefsStatus === "saving";
  const prefsBusy = uiPrefsStatus === "loading" || uiPrefsStatus === "saving";
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
    const list = await api.listDbBackups();
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
    void link.refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- Verbindung default + reload when sheet opens
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

  const connecting =
    linkBusy ||
    link.rfcommStatus === "connecting" ||
    link.rfcommStatus === "discovering" ||
    link.rfcommStatus === "reconnecting" ||
    link.status === "searching";

  const hasKnownDevice = link.hasTarget;
  const canSearchDevice = Boolean(link.rfcommFeature && onSearchDevice);
  const canReconnect = Boolean(
    link.rfcommFeature && hasKnownDevice && !link.linked && !connecting,
  );
  const canForget = Boolean(link.rfcommFeature && hasKnownDevice && !connecting);

  const onReconnect = () => {
    if (!canReconnect) return;
    void (async () => {
      setLinkBusy(true);
      setError(null);
      try {
        await liveApi.rfcommConnectReddot();
        await link.refresh();
      } catch (e) {
        setError(String(e));
        await link.refresh();
      } finally {
        setLinkBusy(false);
      }
    })();
  };

  const onForgetDevice = () => {
    if (!canForget) return;
    const name = link.targetName?.trim() || "das bekannte Gerät";
    void (async () => {
      const ok = await confirmDialog({
        title: "Gerät vergessen?",
        body: `„${name}“ vergessen?\n\nDie Arena merkt sich dieses Gerät danach nicht mehr. Du kannst später erneut ein Gerät suchen.`,
        confirmLabel: "Vergessen",
        danger: true,
        eyebrow: "Verbindung",
      });
      if (!ok) return;
      setLinkBusy(true);
      setError(null);
      try {
        await liveApi.rfcommForgetTarget();
        await link.refresh();
      } catch (e) {
        setError(String(e));
        await link.refresh();
      } finally {
        setLinkBusy(false);
      }
    })();
  };

  const deviceLabel = hasKnownDevice
    ? link.targetName?.trim() || "Bekanntes Gerät"
    : "Kein Gerät ausgewählt";
  const statusLabel = connectionStatusLabel({
    linked: link.linked,
    hasTarget: hasKnownDevice,
    connecting,
  });
  const statusTone = connectionStatusTone({
    linked: link.linked,
    hasTarget: hasKnownDevice,
    connecting,
  });

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

      <SettingsSection
        title="Allgemein"
        description="Grundlegende Einstellungen für die App."
        open={openSection === "allgemein"}
        onOpenChange={() => toggleSection("allgemein")}
      >
        {uiPrefsStatus === "loading" ? (
          <SettingsHint>Einstellungen werden geladen…</SettingsHint>
        ) : null}
        {uiPrefsError ? (
          <p className="banner-error">
            {uiPrefsError}
            {onRetryUiPrefs ? (
              <>
                {" "}
                <button type="button" className="ghost" onClick={onRetryUiPrefs}>
                  Erneut laden
                </button>
              </>
            ) : null}
          </p>
        ) : null}
        {uiPrefsStatus === "error" && !uiPrefsError ? (
          <p className="banner-error">Nicht gespeichert.</p>
        ) : null}
        <SettingsChoice
          label="Startansicht"
          value={uiPrefs.startView}
          disabled={!prefsReady}
          options={[
            { value: "live", label: "Arena" },
            { value: "history", label: "Statistik" },
            { value: "bureau", label: "Verwaltung" },
          ]}
          onChange={(value) =>
            onUpdateUiPrefs({
              startView: value as UiPrefs["startView"],
            })
          }
        />
        <SettingsToggle
          label="Letzte Ansicht merken"
          checked={uiPrefs.rememberLastView}
          disabled={!prefsReady}
          onChange={(on) => {
            if (on) {
              onUpdateUiPrefs({
                rememberLastView: true,
                lastView: currentView,
              });
            } else {
              onUpdateUiPrefs({ rememberLastView: false });
            }
          }}
        />
        <SettingsToggle
          label="Kompakte Oberfläche"
          checked={uiPrefs.compactUi}
          disabled={!prefsReady}
          onChange={(on) => onUpdateUiPrefs({ compactUi: on })}
        />
        <SettingsToggle
          label="Größere Schrift"
          hint="Bessere Lesbarkeit auf Distanz."
          checked={uiPrefs.largeText}
          disabled={!prefsReady}
          onChange={(on) => onUpdateUiPrefs({ largeText: on })}
        />
        {prefsBusy && prefsReady ? (
          <SettingsHint>Speichere…</SettingsHint>
        ) : null}
      </SettingsSection>

      <SettingsSection
        title="App & Updates"
        description="Version, Prüfung und Installation von Updates."
        open={openSection === "app"}
        onOpenChange={() => toggleSection("app")}
      >
        <div className="settings-info-block">
          <SettingsInfoRow
            label="Aktuelle Version"
            value={appUpdate.version ?? "…"}
          />
          <SettingsInfoRow
            label="Update-Status"
            value={updateStatusLabel(appUpdate.status)}
            statusTone={updateStatusTone(appUpdate.status)}
          />
        </div>
        {appUpdate.status.kind === "error" ? (
          <p className="banner-error">{appUpdate.status.message}</p>
        ) : null}
        {appUpdate.status.kind === "needsManualRestart" ? (
          <p className="banner-error">{appUpdate.status.message}</p>
        ) : null}
        {appUpdate.status.kind === "devOnly" ? (
          <SettingsHint>
            Updates sind in der Entwicklungsansicht nicht verfügbar. Bitte eine
            installierte App-Version verwenden.
          </SettingsHint>
        ) : appUpdate.status.kind === "available" ? (
          <SettingsHint>
            Version {appUpdate.status.update.version} ist verfügbar. Download
            und Installation starten nur nach Ihrer Bestätigung.
          </SettingsHint>
        ) : appUpdate.status.kind === "readyToRelaunch" ? (
          <SettingsHint>
            Update {appUpdate.status.update.version} ist installiert, aber noch
            nicht aktiv. Bitte neu starten — die laufende Sitzung zeigt weiter
            die alte Version.
          </SettingsHint>
        ) : appUpdate.status.kind === "downloading" ||
          appUpdate.status.kind === "installing" ? (
          <SettingsHint>
            Update läuft — Fortschritt im Update-Fenster. Unter Windows schließt
            sich die App nach dem Download und startet neu.
          </SettingsHint>
        ) : appUpdate.status.kind === "needsManualRestart" ? (
          <SettingsHint>
            Das Update ist installiert. Bitte die App manuell schließen und
            erneut öffnen.
          </SettingsHint>
        ) : (
          <SettingsHint>
            Prüfung nur manuell. Download und Installation nach Bestätigung —
            Fortschritt erscheint in einem eigenen Fenster.
          </SettingsHint>
        )}
        <div className="settings-connection-actions">
          {appUpdate.status.kind === "available" ? (
            <button
              type="button"
              className="settings-action-primary"
              disabled={appUpdate.busy}
              onClick={() => {
                if (appUpdate.status.kind !== "available") return;
                const ver = appUpdate.status.update.version;
                void (async () => {
                  const ok = await confirmDialog({
                    title: "Update installieren?",
                    body: `Update ${ver} herunterladen und installieren?\n\nDie App schließt sich danach und startet neu.`,
                    confirmLabel: "Installieren",
                    eyebrow: "App-Update",
                  });
                  if (!ok) return;
                  appUpdate.beginInstallFromUi();
                })();
              }}
            >
              Update installieren
            </button>
          ) : null}
          <button
            type="button"
            className={
              appUpdate.status.kind === "available"
                ? "secondary settings-action-secondary"
                : "settings-action-primary"
            }
            disabled={appUpdate.busy}
            onClick={() => void appUpdate.checkForUpdates()}
          >
            {appUpdate.checking ? "Suche…" : "Nach Updates suchen"}
          </button>
          {appUpdate.status.kind === "readyToRelaunch" ||
          appUpdate.status.kind === "needsManualRestart" ? (
            <button
              type="button"
              className="settings-action-primary"
              disabled={appUpdate.busy}
              onClick={() => void appUpdate.relaunchToApply()}
            >
              Jetzt neu starten
            </button>
          ) : null}
        </div>
      </SettingsSection>

      <SettingsSection
        title="Darstellung"
        description="Passe das Erscheinungsbild der App an."
        open={openSection === "darstellung"}
        onOpenChange={() => toggleSection("darstellung")}
      >
        {uiPrefsStatus === "loading" ? (
          <SettingsHint>Einstellungen werden geladen…</SettingsHint>
        ) : null}
        <SettingsChoice
          label="Farbschema"
          value={uiPrefs.colorScheme}
          disabled={!prefsReady}
          options={[
            { value: "system", label: "System" },
            { value: "light", label: "Hell" },
            { value: "dark", label: "Dunkel" },
          ]}
          onChange={(value) =>
            onUpdateUiPrefs({
              colorScheme: value as UiPrefs["colorScheme"],
            })
          }
        />
        <SettingsToggle
          label="Reduzierte Bewegungen"
          hint="Weniger visuelle Bewegung und ruhigere Übergänge."
          checked={uiPrefs.reducedMotion}
          disabled={!prefsReady}
          onChange={(on) => onUpdateUiPrefs({ reducedMotion: on })}
        />
        <SettingsToggle
          label="Extra große Schrift & Buttons"
          hint="Deutlich größere Typo und Bedienelemente — gut aus Distanz / in der Halle."
          checked={uiPrefs.extraLargeUi}
          disabled={!prefsReady}
          onChange={(on) => onUpdateUiPrefs({ extraLargeUi: on })}
        />
        <SettingsHint>
          Hell/Dunkel steuert Farben; System folgt der Betriebssystem-Einstellung.
        </SettingsHint>
      </SettingsSection>

      <SettingsSection
        title="Arena"
        description="Verhalten und Darstellung in der Arena."
        open={openSection === "arena"}
        onOpenChange={() => toggleSection("arena")}
      >
        {uiPrefsStatus === "loading" ? (
          <SettingsHint>Einstellungen werden geladen…</SettingsHint>
        ) : null}
        <SettingsChoice
          label="Trefferanzeige"
          value={uiPrefs.scoreDisplay}
          disabled={!prefsReady}
          options={[
            { value: "punkte", label: "Punkte zuerst" },
            { value: "teiler", label: "Teiler zuerst" },
          ]}
          onChange={(value) =>
            onUpdateUiPrefs({
              scoreDisplay: value as UiPrefs["scoreDisplay"],
            })
          }
        />
        <SettingsChoice
          label="Trefferfeedback"
          value={uiPrefs.hitFeedback}
          disabled={!prefsReady}
          options={[
            { value: "normal", label: "Normal" },
            { value: "reduced", label: "Reduziert" },
            { value: "minimal", label: "Minimal" },
          ]}
          onChange={(value) =>
            onUpdateUiPrefs({
              hitFeedback: value as UiPrefs["hitFeedback"],
            })
          }
        />
        <SettingsChoice
          label="Zieldarstellung"
          value={uiPrefs.targetFit}
          disabled={!prefsReady}
          options={[
            { value: "auto", label: "Automatisch" },
            { value: "calm", label: "Ruhig" },
            { value: "aggressive", label: "Aggressiv" },
          ]}
          hint="Wie stark die Scheibe an das Fenster angepasst wird."
          onChange={(value) =>
            onUpdateUiPrefs({
              targetFit: value as UiPrefs["targetFit"],
            })
          }
        />
        <SettingsToggle
          label="Letzte Wertungsansicht merken"
          hint="Manuelle Umschaltung Punkte/Teiler im Training speichern."
          checked={uiPrefs.rememberScoreDisplay}
          disabled={!prefsReady}
          onChange={(on) => onUpdateUiPrefs({ rememberScoreDisplay: on })}
        />
        <SettingsHint>
          Im Wettkampf gilt die Wertungsart des Wettbewerbs — die Nutzerpräferenz
          wird nicht überschrieben.
        </SettingsHint>
      </SettingsSection>

      <SettingsSection
        title="Verbindung"
        description="Verwalte das bekannte Gerät und den Verbindungsstatus."
        open={openSection === "verbindung"}
        onOpenChange={() => toggleSection("verbindung")}
      >
        <div className="settings-info-block">
          <SettingsInfoRow label="Bekanntes Gerät" value={deviceLabel} />
          <SettingsInfoRow
            label="Status"
            value={statusLabel}
            statusTone={statusTone}
          />
        </div>
        {connecting ? (
          <SettingsHint>Verbindung wird hergestellt…</SettingsHint>
        ) : !hasKnownDevice ? (
          <SettingsHint>
            Suche nach verfügbaren RedDot-Geräten in der Nähe.
          </SettingsHint>
        ) : null}
        <div className="settings-connection-actions">
          <button
            type="button"
            className="settings-action-primary"
            disabled={!canSearchDevice || linkBusy}
            onClick={() => onSearchDevice?.()}
          >
            {hasKnownDevice ? "Anderes Gerät verbinden" : "Gerät suchen"}
          </button>
          <button
            type="button"
            className="secondary settings-action-secondary"
            disabled={!canReconnect}
            title={
              !hasKnownDevice
                ? "Zuerst ein Gerät suchen"
                : link.linked
                  ? "Bereits verbunden"
                  : connecting
                    ? "Verbindung läuft"
                    : undefined
            }
            onClick={onReconnect}
          >
            Neu verbinden
          </button>
          <button
            type="button"
            className="ghost settings-action-quiet"
            disabled={!canForget}
            title={
              !hasKnownDevice
                ? "Kein Gerät gespeichert"
                : connecting
                  ? "Bitte warten, bis die Verbindung abgeschlossen ist"
                  : undefined
            }
            onClick={onForgetDevice}
          >
            Gerät vergessen
          </button>
        </div>
      </SettingsSection>

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
