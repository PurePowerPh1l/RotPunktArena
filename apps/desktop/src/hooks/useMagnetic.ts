import { useCallback, useRef, type PointerEvent } from "react";

type Options = {
  strength?: number;
  disabled?: boolean;
};

/** Soft cursor magnetism — Apple-style “pull” without Framer. */
export function useMagnetic({ strength = 0.28, disabled = false }: Options = {}) {
  const ref = useRef<HTMLButtonElement | null>(null);
  const raf = useRef(0);

  const onPointerMove = useCallback(
    (e: PointerEvent<HTMLButtonElement>) => {
      if (disabled) return;
      const el = ref.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const x = e.clientX - rect.left - rect.width / 2;
      const y = e.clientY - rect.top - rect.height / 2;
      cancelAnimationFrame(raf.current);
      raf.current = requestAnimationFrame(() => {
        el.style.transform = `translate3d(${x * strength}px, ${y * strength}px, 0) scale(1.03)`;
      });
    },
    [disabled, strength],
  );

  const reset = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    cancelAnimationFrame(raf.current);
    el.style.transform = "";
  }, []);

  return { ref, onPointerMove, onPointerLeave: reset, onPointerCancel: reset };
}
