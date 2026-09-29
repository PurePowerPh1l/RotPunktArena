import { useLayoutEffect, useRef, type ReactNode } from "react";

type Props = {
  title: string;
  ariaLabel: string;
  onClose: () => void;
  children: ReactNode;
  /** Shift left when another sheet is also open (same anchor). */
  stackedSecondary?: boolean;
  className?: string;
};

/** Shared layout shell for Settings + Developer side sheets (no access logic). */
export function SideSheetShell({
  title,
  ariaLabel,
  onClose,
  children,
  stackedSecondary = false,
  className,
}: Props) {
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useLayoutEffect(() => {
    const previousFocus = document.activeElement;
    closeButtonRef.current?.focus();
    return () => {
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) {
        previousFocus.focus();
      }
    };
  }, []);

  const classes = [
    "side-sheet",
    stackedSecondary ? "is-stacked-secondary" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <aside className={classes} aria-label={ariaLabel}>
      <div className="side-sheet-head">
        <strong>{title}</strong>
        <button ref={closeButtonRef} type="button" className="ghost" onClick={onClose}>
          Schließen
        </button>
      </div>
      {children}
    </aside>
  );
}

type SectionProps = {
  label: string;
  danger?: boolean;
  children: ReactNode;
};

export function SideSheetSection({ label, danger, children }: SectionProps) {
  return (
    <div className={`side-sheet-section${danger ? " side-sheet-danger" : ""}`}>
      <p className="side-sheet-label">{label}</p>
      {children}
    </div>
  );
}
