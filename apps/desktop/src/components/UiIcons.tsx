import type { ReactNode } from "react";

/** Minimal custom UI icons — line style, no emoji. */

type IconProps = {
  size?: number;
  className?: string;
};

function Svg({
  size = 16,
  className,
  children,
}: IconProps & { children: ReactNode }) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function IconSound({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M4 9.5v5h3.2L12 18V6L7.2 9.5H4z" fill="currentColor" stroke="none" />
      <path d="M15.2 9.2a3.2 3.2 0 0 1 0 5.6" />
      <path d="M17.6 6.8a6.2 6.2 0 0 1 0 10.4" />
    </Svg>
  );
}

export function IconMute({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M4 9.5v5h3.2L12 18V6L7.2 9.5H4z" fill="currentColor" stroke="none" />
      <path d="M16 9.5l5 5M21 9.5l-5 5" />
    </Svg>
  );
}

export function IconTraining({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <circle cx="12" cy="12" r="8.5" />
      <circle cx="12" cy="12" r="5" />
      <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none" />
    </Svg>
  );
}

/** Nav: Arena — Scheibe mit Fadenkreuz und Treffer. */
export function IconArena({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <circle cx="12" cy="12" r="7.8" />
      <circle cx="12" cy="12" r="4.2" />
      <path d="M12 2.6v3.1M12 18.3v3.1M2.6 12h3.1M18.3 12h3.1" />
      <circle cx="14.35" cy="9.55" r="1.45" fill="currentColor" stroke="none" />
    </Svg>
  );
}

/** Nav: Statistik — steigende Balken mit Trend. */
export function IconStats({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M5 19.2V11.4" strokeWidth="2.15" />
      <path d="M10.2 19.2V8.2" strokeWidth="2.15" />
      <path d="M15.4 19.2V5.4" strokeWidth="2.15" />
      <path d="M4.4 13.6 9.6 10.2 14.8 6.6 19.4 4.4" />
      <circle cx="19.4" cy="4.4" r="1.25" fill="currentColor" stroke="none" />
    </Svg>
  );
}

/** Nav: Verwaltung — Clipboard mit Person und Liste. */
export function IconAdmin({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <rect x="5.4" y="5.8" width="13.2" height="14.4" rx="1.6" />
      <path d="M9 5.8V4.6c0-.7.55-1.25 1.25-1.25h3.5c.7 0 1.25.55 1.25 1.25v1.2" />
      <circle cx="9.35" cy="11.15" r="1.55" />
      <path d="M7.2 16.4c.55-1.55 1.45-2.25 2.9-2.25" />
      <path d="M13.1 10.9h4M13.1 14h4M13.1 17.1h2.6" />
    </Svg>
  );
}

/** Settings — Zahnrad. */
export function IconSettings({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </Svg>
  );
}

export function IconPlay({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M8 6.5v11l10-5.5z" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconDev({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M14.5 5.2a3.8 3.8 0 0 0-5.2 5.2L4 15.7 6.3 18l5.3-5.3a3.8 3.8 0 0 0 5.2-5.2l-2.1 2.1-2.2-2.2 2-2.2z" />
    </Svg>
  );
}

export function IconPlug({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M9 4v5M15 4v5" />
      <path d="M7.5 9h9v4.2a4.5 4.5 0 0 1-4.5 4.5h0a4.5 4.5 0 0 1-4.5-4.5V9z" />
      <path d="M12 17.7V20.5" />
    </Svg>
  );
}

export function IconStop({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <rect x="6.5" y="6.5" width="11" height="11" rx="1.2" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconPrint({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M7 9V4.5h10V9" />
      <path d="M6 13.5h12v6.2H6z" />
      <path d="M5 9h14v5.2H5z" />
      <path d="M15.5 15.8h2" />
    </Svg>
  );
}

export function IconTrophy({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M8 5h8v3.2a4 4 0 0 1-8 0V5z" />
      <path d="M8 6.2H5.2a2.2 2.2 0 0 0 2.2 2.8" />
      <path d="M16 6.2h2.8a2.2 2.2 0 0 1-2.2 2.8" />
      <path d="M12 12.2v2.3" />
      <path d="M9 19.2h6M10.2 14.5h3.6V19" />
    </Svg>
  );
}

export function IconPerson({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <circle cx="12" cy="8" r="3.2" />
      <path d="M5.5 19.2c1.4-3.2 3.6-4.8 6.5-4.8s5.1 1.6 6.5 4.8" />
    </Svg>
  );
}

export function IconCheck({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M8.2 12.2l2.6 2.6 5-5.2" />
    </Svg>
  );
}

/** Enter fullscreen */
export function IconExpand({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M8 4.5H4.5V8M16 4.5h3.5V8M8 19.5H4.5V16M16 19.5h3.5V16" />
    </Svg>
  );
}

export function IconCompress({ size, className }: IconProps) {
  return (
    <Svg size={size} className={className}>
      <path d="M9.5 4.5v4H5.5M14.5 4.5v4h4M9.5 19.5v-4H5.5M14.5 19.5v-4h4" />
    </Svg>
  );
}
