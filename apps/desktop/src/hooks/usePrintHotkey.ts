import { useEffect } from "react";

/**
 * Ctrl/Cmd+P → custom print handler (prevents blank page print).
 * Pass `null`/`undefined` to disable while another surface owns print.
 */
export function usePrintHotkey(onPrint: (() => void) | null | undefined): void {
  useEffect(() => {
    if (!onPrint) return;
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.key.toLowerCase() !== "p") return;
      const t = e.target as HTMLElement | null;
      if (
        t &&
        (t.tagName === "INPUT" ||
          t.tagName === "TEXTAREA" ||
          t.tagName === "SELECT" ||
          t.isContentEditable)
      ) {
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      onPrint();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onPrint]);
}
