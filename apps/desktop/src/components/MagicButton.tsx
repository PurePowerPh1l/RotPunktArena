import {
  type ButtonHTMLAttributes,
  type MouseEvent,
  useCallback,
  useRef,
} from "react";
import { useMagnetic } from "../hooks/useMagnetic";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  magnetic?: boolean;
  shimmer?: boolean;
};

/** Primary CTA with magnetic hover, spring press, and click ripple. */
export function MagicButton({
  magnetic = true,
  shimmer = true,
  className = "",
  disabled,
  onClick,
  children,
  ...rest
}: Props) {
  const mag = useMagnetic({ disabled: Boolean(disabled) || !magnetic });
  const rippleHost = useRef<HTMLSpanElement>(null);

  const handleClick = useCallback(
    (e: MouseEvent<HTMLButtonElement>) => {
      const host = rippleHost.current;
      if (host && !disabled) {
        const rect = e.currentTarget.getBoundingClientRect();
        const size = Math.max(rect.width, rect.height) * 1.35;
        const x = e.clientX - rect.left - size / 2;
        const y = e.clientY - rect.top - size / 2;
        const wave = document.createElement("span");
        wave.className = "btn-ripple";
        wave.style.width = `${size}px`;
        wave.style.height = `${size}px`;
        wave.style.left = `${x}px`;
        wave.style.top = `${y}px`;
        host.appendChild(wave);
        window.setTimeout(() => wave.remove(), 620);
      }
      onClick?.(e);
    },
    [disabled, onClick],
  );

  const classes = [
    "magic-btn",
    shimmer ? "magic-btn-shimmer" : "",
    magnetic ? "magic-btn-magnetic" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      {...rest}
      ref={mag.ref}
      disabled={disabled}
      className={classes}
      onPointerMove={magnetic ? mag.onPointerMove : undefined}
      onPointerLeave={magnetic ? mag.onPointerLeave : undefined}
      onPointerCancel={magnetic ? mag.onPointerCancel : undefined}
      onClick={handleClick}
    >
      <span className="magic-btn-label">{children}</span>
      <span className="magic-btn-ripples" ref={rippleHost} aria-hidden />
    </button>
  );
}
