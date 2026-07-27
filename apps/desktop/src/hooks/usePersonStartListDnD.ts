import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";

export type PersonPointerDrag = {
  personId: string;
  label: string;
};

type Ghost = PersonPointerDrag & { x: number; y: number };

const DRAG_THRESHOLD_PX = 6;

/**
 * Pointer-based person → start-list drag (HTML5 DnD is unreliable in Tauri/WebView2).
 */
export function usePersonStartListDnD(opts: {
  enabled: boolean;
  onDropPerson: (personId: string) => void | Promise<unknown>;
}) {
  const { enabled, onDropPerson } = opts;
  const [ghost, setGhost] = useState<Ghost | null>(null);
  const [overDrop, setOverDrop] = useState(false);
  const pendingRef = useRef<{
    personId: string;
    label: string;
    x0: number;
    y0: number;
    pointerId: number;
  } | null>(null);
  const draggingRef = useRef(false);
  const overDropRef = useRef(false);
  const onDropRef = useRef(onDropPerson);
  onDropRef.current = onDropPerson;

  const hitStartList = (x: number, y: number) => {
    const el = document.elementFromPoint(x, y);
    return Boolean(el?.closest("[data-start-list-drop='true']"));
  };

  const endDrag = () => {
    pendingRef.current = null;
    draggingRef.current = false;
    overDropRef.current = false;
    setGhost(null);
    setOverDrop(false);
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

      const over = hitStartList(e.clientX, e.clientY);
      overDropRef.current = over;
      setGhost({
        personId: pending.personId,
        label: pending.label,
        x: e.clientX,
        y: e.clientY,
      });
      setOverDrop(over);
    };

    const onUp = (e: PointerEvent) => {
      const pending = pendingRef.current;
      if (!pending || e.pointerId !== pending.pointerId) return;

      const wasDragging = draggingRef.current;
      const over = wasDragging && hitStartList(e.clientX, e.clientY);
      const personId = pending.personId;
      endDrag();

      if (wasDragging && over && enabled) {
        void onDropRef.current(personId);
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

  const beginPersonDrag = (
    personId: string,
    label: string,
    e: ReactPointerEvent,
  ) => {
    if (!enabled) return;
    if (e.button !== 0) return;
    pendingRef.current = {
      personId,
      label,
      x0: e.clientX,
      y0: e.clientY,
      pointerId: e.pointerId,
    };
    draggingRef.current = false;
  };

  return {
    ghost,
    overDrop,
    dragging: ghost != null,
    beginPersonDrag,
    cancelPersonDrag: endDrag,
  };
}
