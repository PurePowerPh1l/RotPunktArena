import type { LeagueRank, LeagueTierId } from "../training/league";

type Props = {
  rank: LeagueRank;
  /** compact = dropdown icon only; detail = icon + label */
  size?: "sm" | "md";
  showLabel?: boolean;
  className?: string;
};

const TIER_TITLE: Record<LeagueTierId, string> = {
  unranked: "Noch in der Platzierung",
  bronze: "Bronze",
  silver: "Silber",
  gold: "Gold",
  platinum: "Platin",
  diamond: "Diamant",
  master: "Meister",
  grandmaster: "Großmeister",
  champion: "Champion",
};

/** Small metallic rank mark for training UI. */
export function LeagueBadge({
  rank,
  size = "sm",
  showLabel = false,
  className,
}: Props) {
  const px = size === "md" ? 22 : 16;
  const title =
    rank.tier === "unranked"
      ? `Platzierung · noch ${rank.placementLeft} Serie${rank.placementLeft === 1 ? "" : "n"}`
      : `${rank.label} · ${rank.sr} SR`;

  return (
    <span
      className={`league-badge league-${rank.tier}${className ? ` ${className}` : ""}`}
      title={title}
      aria-label={title}
    >
      <LeagueMark tier={rank.tier} size={px} />
      {showLabel ? (
        <span className="league-badge-label">
          {rank.tier === "unranked" ? "Platzierung" : rank.label}
        </span>
      ) : null}
    </span>
  );
}

function LeagueMark({ tier, size }: { tier: LeagueTierId; size: number }) {
  // Filled shield / gem marks — color via CSS currentColor on .league-{tier}
  return (
    <svg
      className="league-mark"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      {tier === "unranked" ? (
        <>
          <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeDasharray="2.5 2" />
          <circle cx="12" cy="12" r="2" fill="currentColor" />
        </>
      ) : tier === "diamond" || tier === "champion" ? (
        <path
          d="M12 2.5 L20 9.2 L12 21.5 L4 9.2 Z"
          fill="currentColor"
          stroke="rgba(0,0,0,0.25)"
          strokeWidth="0.8"
        />
      ) : tier === "grandmaster" || tier === "master" ? (
        <path
          d="M12 2.2l2.4 5.1 5.5.5-4.2 3.7 1.3 5.4L12 14.6 7 16.9l1.3-5.4L4.1 7.8l5.5-.5L12 2.2z"
          fill="currentColor"
          stroke="rgba(0,0,0,0.2)"
          strokeWidth="0.7"
        />
      ) : (
        <path
          d="M12 2.5c2.8 1.6 5.2 2 7.5 2.1V11c0 5.2-3.4 8.6-7.5 10.4C7.9 19.6 4.5 16.2 4.5 11V4.6C6.8 4.5 9.2 4.1 12 2.5z"
          fill="currentColor"
          stroke="rgba(0,0,0,0.22)"
          strokeWidth="0.75"
        />
      )}
    </svg>
  );
}

export function leagueTierTitle(tier: LeagueTierId): string {
  return TIER_TITLE[tier];
}
