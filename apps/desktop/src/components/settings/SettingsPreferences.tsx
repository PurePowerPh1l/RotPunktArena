import type { UiPrefs } from "@rotpunktarena/domain";
import type { AppView } from "../appNav";
import type { UiPrefsStatus } from "../../hooks/useUiPrefs";
import { SettingsChoice, SettingsHint, SettingsSection, SettingsToggle } from "./SettingsParts";

type Common = {
  open: boolean;
  onOpenChange: () => void;
  uiPrefs: UiPrefs;
  uiPrefsStatus: UiPrefsStatus;
  onUpdateUiPrefs: (patch: Partial<UiPrefs>) => void;
};


export function GeneralSettings({ open, onOpenChange, uiPrefs, uiPrefsStatus, uiPrefsError, onRetryUiPrefs, onUpdateUiPrefs, currentView }: Common & {
  uiPrefsError: string | null;
  onRetryUiPrefs?: () => void;
  currentView: AppView;
}) {
  const prefsReady = uiPrefsStatus === "ready" || uiPrefsStatus === "saving";
  const prefsBusy = uiPrefsStatus === "loading" || uiPrefsStatus === "saving";
  return (
      <SettingsSection
        title="Allgemein"
        description="Grundlegende Einstellungen für die App."
        open={open}
        onOpenChange={onOpenChange}
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
  );
}

export function AppearanceSettings({ open, onOpenChange, uiPrefs, uiPrefsStatus, onUpdateUiPrefs }: Common) {
  const prefsReady = uiPrefsStatus === "ready" || uiPrefsStatus === "saving";
  return (
      <SettingsSection
        title="Darstellung"
        description="Passe das Erscheinungsbild der App an."
        open={open}
        onOpenChange={onOpenChange}
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
  );
}

export function ArenaSettings({ open, onOpenChange, uiPrefs, uiPrefsStatus, onUpdateUiPrefs }: Common) {
  const prefsReady = uiPrefsStatus === "ready" || uiPrefsStatus === "saving";
  return (
      <SettingsSection
        title="Arena"
        description="Verhalten und Darstellung in der Arena."
        open={open}
        onOpenChange={onOpenChange}
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
  );
}
