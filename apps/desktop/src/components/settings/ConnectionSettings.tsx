import { useEffect, useState } from "react";
import * as liveApi from "../../api/live";
import { useLiveLinkStatus } from "../../hooks/useLiveLinkStatus";
import { confirmDialog } from "../../hooks/useAppDialog";
import { SettingsHint, SettingsInfoRow, SettingsSection, type SettingsStatusTone } from "./SettingsParts";

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

export function ConnectionSettings({ open, onOpenChange, onSearchDevice }: {
  open: boolean;
  onOpenChange: () => void;
  onSearchDevice?: () => void;
}) {
  const link = useLiveLinkStatus();
  const [linkBusy, setLinkBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [knownDevices, setKnownDevices] = useState<liveApi.KnownDevice[]>([]);
  const reloadKnownDevices = async () => {
    if (!link.rfcommFeature) {
      setKnownDevices([]);
      return;
    }
    const list = await liveApi.rfcommListDevices();
    setKnownDevices(list);
  };

  useEffect(() => {
    void link.refresh();
    void reloadKnownDevices().catch((e) => setError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refresh on mount
  }, []);

  useEffect(() => {
    if (!link.rfcommFeature) return;
    void reloadKnownDevices().catch((e) => setError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refresh after device switch
  }, [link.rfcommFeature, link.hasTarget, link.linked, link.targetName]);

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
        await reloadKnownDevices();
      } catch (e) {
        setError(String(e));
        await link.refresh();
      } finally {
        setLinkBusy(false);
      }
    })();
  };

  const onSwitchKnownDevice = (device: liveApi.KnownDevice) => {
    if (linkBusy || connecting) return;
    if (device.isActive && link.linked) return;
    void (async () => {
      setLinkBusy(true);
      setError(null);
      try {
        if (device.isActive) {
          await liveApi.rfcommConnectReddot();
        } else {
          await liveApi.rfcommSetupConnect(device.btAddrHex, device.displayName);
        }
        await link.refresh();
        await reloadKnownDevices();
      } catch (e) {
        setError(String(e));
        await link.refresh();
        await reloadKnownDevices().catch(() => {});
      } finally {
        setLinkBusy(false);
      }
    })();
  };

  const onForgetKnownDevice = (device: liveApi.KnownDevice) => {
    if (linkBusy || connecting) return;
    void (async () => {
      const ok = await confirmDialog({
        title: "Gerät vergessen?",
        body: `„${device.displayName}“ aus dem Gerätegedächtnis entfernen?`,
        confirmLabel: "Vergessen",
        danger: true,
        eyebrow: "Verbindung",
      });
      if (!ok) return;
      setLinkBusy(true);
      setError(null);
      try {
        await liveApi.rfcommForgetDevice(device.btAddrHex);
        await link.refresh();
        await reloadKnownDevices();
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

  return (
    <>
      {error ? <p className="banner-error" role="alert">{error}</p> : null}
      <SettingsSection
        title="Verbindung"
        description="Gerätegedächtnis und Verbindungsstatus."
        open={open}
        onOpenChange={onOpenChange}
      >
        <div className="settings-info-block">
          <SettingsInfoRow label="Aktives Gerät" value={deviceLabel} />
          <SettingsInfoRow
            label="Status"
            value={statusLabel}
            statusTone={statusTone}
          />
        </div>
        {knownDevices.length > 0 ? (
          <ul className="settings-device-list">
            {knownDevices.map((device) => {
              const hint = device.isActive
                ? link.linked
                  ? "Verbunden"
                  : "Zuletzt verbunden — tippe zum Reparieren"
                : "Gemerkt — tippe zum Wechseln";
              const canTap =
                !linkBusy &&
                !connecting &&
                !(device.isActive && link.linked);
              return (
                <li key={device.btAddrHex} className="settings-device-row">
                  <button
                    type="button"
                    className="settings-device-main"
                    disabled={!canTap}
                    title={
                      device.isActive && link.linked
                        ? "Bereits verbunden"
                        : device.isActive
                          ? "Verbindung reparieren"
                          : "Dieses Gerät verbinden"
                    }
                    onClick={() => onSwitchKnownDevice(device)}
                  >
                    <span className="settings-device-name">
                      {device.displayName}
                    </span>
                    <span className="settings-device-hint muted">{hint}</span>
                  </button>
                  <button
                    type="button"
                    className="ghost settings-device-forget"
                    disabled={linkBusy || connecting}
                    title="Aus dem Gerätegedächtnis entfernen"
                    onClick={() => onForgetKnownDevice(device)}
                  >
                    Vergessen
                  </button>
                </li>
              );
            })}
          </ul>
        ) : null}
        {connecting ? (
          <SettingsHint>Verbindung wird hergestellt…</SettingsHint>
        ) : knownDevices.length === 0 ? (
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
            {hasKnownDevice || knownDevices.length > 0
              ? "Anderes Gerät verbinden"
              : "Gerät suchen"}
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
          {knownDevices.length === 0 ? (
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
          ) : null}
        </div>
      </SettingsSection>
    </>
  );
}
