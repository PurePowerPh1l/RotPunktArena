import type { ReactNode } from "react";
import {
  APP_NAV_OPTIONS,
  navigateAppView,
  type AppView,
} from "./appNav";
import { BrandLogo } from "./BrandLogo";
import { LiveLinkBadge } from "./LiveLinkBadge";
import { LiveStandTip } from "./LiveStandTip";
import { SlidingSeg } from "./SlidingSeg";
import { IconCompress, IconExpand } from "./UiIcons";

type NavHandlers = {
  onOpenLive: () => void;
  onOpenBureau: () => void;
  onOpenHistory: () => void;
};

type Props = {
  subtitle: string;
  /** Current app place — drives the sliding nav pill. */
  view: AppView;
  nav: NavHandlers;
  /** Extra tools before fullscreen (sound / Dev). */
  tools?: ReactNode;
  fullscreen?: boolean;
  onToggleFullscreen?: () => void;
  /** Dad-joke tip next to the brand (hidden e.g. in Arena × Wettkampf). */
  showStandTip?: boolean;
  /** Competition has Nachkauf — gold chip next to the live-link Dot. */
  nachkaufActive?: boolean;
  /** Badge click when no bond — open first-setup sheet on Live. */
  onRequestSetup?: () => void;
};

/** Shared top bar: brand left, place-nav centered, status/tools right. */
export function AppTopBar({
  subtitle,
  view,
  nav,
  tools,
  fullscreen = false,
  onToggleFullscreen,
  showStandTip = true,
  nachkaufActive = false,
  onRequestSetup,
}: Props) {
  return (
    <header className="top">
      <div className="top-leading">
        <BrandLogo subtitle={subtitle} />
        {showStandTip ? <LiveStandTip /> : null}
      </div>

      <div className="top-nav-center">
        <SlidingSeg
          className="top-nav-seg"
          size="md"
          ariaLabel="Hauptnavigation"
          value={view}
          options={APP_NAV_OPTIONS}
          onChange={(next) => navigateAppView(next, nav)}
        />
      </div>

      <div className="top-actions">
        {/* Link is app-lifetime — badge stays operable in every view
            (collapsed to the dot; expands on hover/focus). */}
        <div className="top-live-slot">
          <LiveLinkBadge onRequestSetup={onRequestSetup} />
          {nachkaufActive ? (
            <span
              className="nachkauf-chip"
              title="Nachkauf aktiv — fertige Starter können erneut starten"
            >
              Nachkauf
            </span>
          ) : null}
        </div>

        <div className="top-cluster" role="group" aria-label="Ansicht">
          {tools}
          {onToggleFullscreen ? (
            <button
              type="button"
              className={`top-tool${fullscreen ? " is-on" : ""}`}
              onClick={onToggleFullscreen}
              title={
                fullscreen
                  ? "Vollbild verlassen (Esc oder Button)"
                  : "Vollbild"
              }
              aria-label={fullscreen ? "Fenster" : "Vollbild"}
              aria-pressed={fullscreen}
            >
              {fullscreen ? <IconCompress /> : <IconExpand />}
            </button>
          ) : null}
        </div>
      </div>
    </header>
  );
}
