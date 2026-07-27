import { useCallback, useRef, useState } from "react";

export type AsyncActionResult<T> =
  | { ok: true; value: T }
  | { ok: false; reason: "busy" | "error"; message?: string };

/** Shared busy + error boilerplate for command-style async actions. */
export function useAsyncAction() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inflight = useRef(false);

  const run = useCallback(async <T,>(fn: () => Promise<T>): Promise<AsyncActionResult<T>> => {
    if (inflight.current) return { ok: false, reason: "busy" };
    inflight.current = true;
    setBusy(true);
    setError(null);
    try {
      return { ok: true, value: await fn() };
    } catch (e) {
      const message = String(e);
      setError(message);
      return { ok: false, reason: "error", message };
    } finally {
      inflight.current = false;
      setBusy(false);
    }
  }, []);

  return { busy, error, setError, run };
}
