import {
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type TransitionEvent,
} from "react";

type Props = {
  open: boolean;
  children: ReactNode;
  className?: string;
  /** Called after the close animation finishes and content unmounts. */
  onExited?: () => void;
  /** Optional: scroll into view when opening. */
  scrollOnOpen?: boolean;
};

/**
 * Height-animate open/close via grid-template-rows (0fr ↔ 1fr).
 * Keeps children mounted until the exit transition ends.
 */
export function ExpandSlot({
  open,
  children,
  className,
  onExited,
  scrollOnOpen = false,
}: Props) {
  const [mounted, setMounted] = useState(open);
  const [shown, setShown] = useState(open);
  const rootRef = useRef<HTMLDivElement>(null);
  const onExitedRef = useRef(onExited);
  onExitedRef.current = onExited;
  const reduceMotion = useRef(
    typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  useEffect(() => {
    if (open) {
      setMounted(true);
      if (reduceMotion.current) {
        setShown(true);
        return;
      }
      const id = requestAnimationFrame(() => {
        requestAnimationFrame(() => setShown(true));
      });
      return () => cancelAnimationFrame(id);
    }
    setShown(false);
    if (reduceMotion.current) {
      setMounted(false);
      onExitedRef.current?.();
    }
  }, [open]);

  useEffect(() => {
    if (!open || !shown || !scrollOnOpen) return;
    const el = rootRef.current;
    if (!el) return;
    const t = window.setTimeout(() => {
      el.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }, 60);
    return () => window.clearTimeout(t);
  }, [open, shown, scrollOnOpen]);

  const onTransitionEnd = (e: TransitionEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    if (
      e.propertyName !== "grid-template-rows" &&
      e.propertyName !== "opacity"
    ) {
      return;
    }
    if (!open) {
      setMounted(false);
      onExitedRef.current?.();
    }
  };

  if (!mounted) return null;

  return (
    <div
      ref={rootRef}
      className={`expand-slot${shown ? " is-open" : ""}${className ? ` ${className}` : ""}`}
      onTransitionEnd={onTransitionEnd}
      aria-hidden={!shown}
    >
      <div className="expand-slot-clip">
        <div className="expand-slot-body">{children}</div>
      </div>
    </div>
  );
}
