import type { ReactNode } from "react";
import type { SegOption } from "./SlidingSeg";
import { IconAdmin, IconArena, IconStats } from "./UiIcons";

export type AppView = "live" | "bureau" | "history";

type NavHandlers = {
  onOpenLive: () => void;
  onOpenBureau: () => void;
  onOpenHistory: () => void;
};

function labelWithIcon(icon: ReactNode, text: string) {
  return (
    <span className="seg-label">
      {icon}
      {text}
    </span>
  );
}

/** Fixed primary places — always all three, stable order. */
export const APP_NAV_OPTIONS: SegOption<AppView>[] = [
  {
    value: "live",
    label: labelWithIcon(<IconArena />, "Arena"),
  },
  {
    value: "history",
    label: labelWithIcon(<IconStats />, "Statistik"),
  },
  {
    value: "bureau",
    label: labelWithIcon(<IconAdmin />, "Verwaltung"),
  },
];

export function navigateAppView(view: AppView, handlers: NavHandlers) {
  if (view === "live") handlers.onOpenLive();
  else if (view === "bureau") handlers.onOpenBureau();
  else handlers.onOpenHistory();
}
