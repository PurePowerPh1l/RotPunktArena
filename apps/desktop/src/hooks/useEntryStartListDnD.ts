import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";

export type EntryPointerDrag = {
  entryId: string;
  label: string;
};

type Ghost = EntryPointerDrag & {
  x: number;
  y: number;
  /** True when pointer is outside the start list (drop = remove). */
  removing: boolean;
};

const DRAG_THRESHOLD_PX = 6;

/**
 * Pointer-based start-list entry drag:
 * - drop on another entry → reorder
 * - drop outside the start list → remove
 * (HTML5 DnD is unreliable in Tauri/WebView2.)
 */
export function useEntryStartListDnD(opts: {
  enabled: boolean;
  onReorder: (draggedId: string, targetId: string) => void | Promise<unknown>;
  onRemove: (entryId: string) => void | Promise<unknown>;
  /** When false, drop outside cancels instead of removing. Default: allow. */
  canRemove?: (entryId: string) => boolean;
}) {
  const { enabled, onReorder, onRemove, canRemove } = opts;
  const [ghost, setGhost] = useState<Ghost | null>(null);
  const [overEntryId, setOverEntryId] = useState<string | null>(null);
  const pendingRef = useRef<{
    entryId: string;
    label: string;
    x0: number;
    y0: number;
    pointerId: number;
  } | null>(null);
  const draggingRef = useRef(false);
  const overEntryRef = useRef<string | null>(null);
  const outsideRef = useRef(false);
  const onReorderRef = useRef(onReorder);
  const onRemoveRef = useRef(onRemove);
  const canRemoveRef = useRef(canRemove);
  onReorderRef.current = onReorder;
  onRemoveRef.current = onRemove;
  canRemoveRef.current = canRemove;

  const hitStartList = (x: number, y: number) => {
    const el = document.elementFromPoint(x, y);
    return Boolean(el?.closest("[data-start-list-drop='true']"));
  };

  const hitEntryId = (x: number, y: number, excludeId: string) => {
    const el = document.elementFromPoint(x, y);
    const row = el?.closest("[data-start-entry]") as HTMLElement | null;
    const id = row?.dataset.startEntry ?? null;
    if (!id || id === excludeId) return null;
    return id;
  };

  const endDrag = () => {
    pendingRef.current = null;
    draggingRef.current = false;
    overEntryRef.current = null;
    outsideRef.current = false;
    setGhost(null);
    setOverEntryId(null);
  };

  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const pending = pendingRef.current;
      if (!pending || e.pointerId !== pending.pointerId) return;

      const dx = e.clientX - pending.x0;
      const dy = e.clientY - pending.y0;
      if (!draggingRef.current) {
        if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
        draggingRef.current = true;
      }

      const inside = hitStartList(e.clientX, e.clientY);
      const allowRemove =
        !inside && (canRemoveRef.current?.(pending.entryId) ?? true);
      const over = inside
        ? hitEntryId(e.clientX, e.clientY, pending.entryId)
        : null;
      outsideRef.current = !inside;
      overEntryRef.current = over;
      setOverEntryId(over);
      setGhost({
        entryId: pending.entryId,
        label: pending.label,
        x: e.clientX,
        y: e.clientY,
        removing: allowRemove,
      });
    };

    const onUp = (e: PointerEvent) => {
      const pending = pendingRef.current;
      if (!pending || e.pointerId !== pending.pointerId) return;

      const wasDragging = draggingRef.current;
      const entryId = pending.entryId;
      const over = overEntryRef.current;
      const outside = outsideRef.current;
      const allowRemove = canRemoveRef.current?.(entryId) ?? true;
      endDrag();

      if (!wasDragging || !enabled) return;
      if (outside) {
        if (allowRemove) void onRemoveRef.current(entryId);
        return;
      }
      if (over) {
        void onReorderRef.current(entryId, over);
      }
    };

    const onCancel = () => endDrag();

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
    };
  }, [enabled]);

  const beginEntryDrag = (
    entryId: string,
    label: string,
    e: ReactPointerEvent,
  ) => {
    if (!enabled) return;
    if (e.button !== 0) return;
    pendingRef.current = {
      entryId,
      label,
      x0: e.clientX,
      y0: e.clientY,
      pointerId: e.pointerId,
    };
    draggingRef.current = false;
  };

  return {
    ghost,
    overEntryId,
    dragging: ghost != null,
    removing: Boolean(ghost?.removing),
    beginEntryDrag,
    cancelEntryDrag: endDrag,
  };
}
