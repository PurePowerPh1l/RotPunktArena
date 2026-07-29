import {
  type ReactNode,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

export type SegOption<T extends string> = {
  value: T;
  label: ReactNode;
};

type Props<T extends string> = {
  options: SegOption<T>[];
  value: T;
  onChange: (next: T) => void;
  className?: string;
  ariaLabel: string;
  disabled?: boolean;
  size?: "sm" | "md";
};

/** iOS-style sliding pill segment control. */
export function SlidingSeg<T extends string>({
  options,
  value,
  onChange,
  className = "",
  ariaLabel,
  disabled = false,
  size = "md",
}: Props<T>) {
  const root = useRef<HTMLDivElement>(null);
  const [pill, setPill] = useState({ left: 0, width: 0, ready: false });

  const optionsKey = options.map((o) => o.value).join("|");

  useLayoutEffect(() => {
    const rootEl = root.current;
    if (!rootEl) return;

    const measure = () => {
      const active = rootEl.querySelector<HTMLElement>("[data-seg-active='true']");
      if (!active) return;
      setPill((prev) => {
        const next = {
          left: active.offsetLeft,
          width: active.offsetWidth,
          ready: true,
        };
        if (
          prev.ready &&
          prev.left === next.left &&
          prev.width === next.width
        ) {
          return prev;
        }
        return next;
      });
    };

    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(rootEl);
    return () => ro.disconnect();
  }, [value, optionsKey]);

  return (
    <div
      ref={root}
      className={`sliding-seg sliding-seg-${size} ${className}`.trim()}
      data-value={value}
      role="group"
      aria-label={ariaLabel}
    >
      <span
        className={`sliding-seg-pill${pill.ready ? " sliding-seg-pill-on" : ""}`}
        style={{
          transform: `translate3d(${pill.left}px, 0, 0)`,
          width: pill.width,
        }}
        aria-hidden
      />
      {options.map((opt) => {
        const on = opt.value === value;
        return (
          <button
            key={opt.value}
            type="button"
            data-seg-active={on ? "true" : "false"}
            className={on ? "sliding-seg-btn on" : "sliding-seg-btn"}
            disabled={disabled}
            aria-pressed={on}
            aria-current={on ? "true" : undefined}
            onClick={() => {
              if (!on) onChange(opt.value);
            }}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
