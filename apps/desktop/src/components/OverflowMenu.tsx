import {
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type MouseEvent,
} from "react";

type Props = {
  /** Button label — default "…" */
  label?: ReactNode;
  /** Accessible name for the trigger */
  ariaLabel?: string;
  className?: string;
  menuClassName?: string;
  disabled?: boolean;
  children: ReactNode;
};

/**
 * Overflow / "Mehr" menu with click-outside + Escape close.
 * Replaces native &lt;details&gt; which only toggles on the summary.
 */
export function OverflowMenu({
  label = "…",
  ariaLabel = "Weitere Aktionen",
  className = "",
  menuClassName = "",
  disabled = false,
  children,
}: Props) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: Event) => {
      const t = e.target as HTMLElement | null;
      if (!t) return;
      // Portal lists (SearchSelect) render outside the menu root.
      if (t.closest(".shooter-ac-list-portal, .shooter-ac-list")) return;
      if (!rootRef.current?.contains(t)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const onMenuClick = (e: MouseEvent) => {
    const t = e.target as HTMLElement | null;
    if (t?.closest("button, a, [data-close-menu]")) setOpen(false);
  };

  return (
    <div
      ref={rootRef}
      className={`row-overflow${open ? " is-open" : ""}${className ? ` ${className}` : ""}`}
    >
      <button
        type="button"
        className="row-overflow-summary"
        aria-label={ariaLabel}
        aria-expanded={open}
        aria-haspopup="menu"
        disabled={disabled}
        onClick={() => setOpen((o) => !o)}
      >
        {label}
      </button>
      {open ? (
        <div
          className={`row-overflow-menu${menuClassName ? ` ${menuClassName}` : ""}`}
          role="menu"
          onClick={onMenuClick}
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}
