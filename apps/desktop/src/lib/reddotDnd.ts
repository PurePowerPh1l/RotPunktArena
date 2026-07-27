import type { DragEvent } from "react";

export type ReddotDragPayload =
  | { kind: "person"; personId: string }
  | { kind: "entry"; entryId: string };

const MIME = "application/x-reddot";
const TEXT = "text/plain";

/**
 * Synchronous store for the in-flight drag.
 * React state alone is too late for the first dragover events
 * (and some WebViews never expose custom MIME types during dragover).
 */
let livePayload: ReddotDragPayload | null = null;

export function peekReddotDrag(): ReddotDragPayload | null {
  return livePayload;
}

export function clearLiveReddotDrag() {
  livePayload = null;
}

export function readReddotDrag(e: DragEvent): ReddotDragPayload | null {
  try {
    const raw = e.dataTransfer.getData(MIME) || e.dataTransfer.getData(TEXT);
    if (raw) {
      const parsed = JSON.parse(raw) as ReddotDragPayload;
      if (parsed?.kind === "person" || parsed?.kind === "entry") return parsed;
    }
  } catch {
    /* fall through to live payload */
  }
  return livePayload;
}

export function writeReddotDrag(e: DragEvent, payload: ReddotDragPayload) {
  livePayload = payload;
  const raw = JSON.stringify(payload);
  try {
    e.dataTransfer.setData(MIME, raw);
  } catch {
    /* some hosts reject custom MIME */
  }
  try {
    e.dataTransfer.setData(TEXT, raw);
  } catch {
    /* ignore */
  }
  e.dataTransfer.effectAllowed = "copyMove";
}

/** True if the current drag is a person being moved onto a start list. */
export function isPersonDrag(
  e: DragEvent,
  active: ReddotDragPayload | null,
): boolean {
  if (active?.kind === "person") return true;
  if (livePayload?.kind === "person") return true;
  const types = Array.from(e.dataTransfer.types ?? []);
  // Prefer our custom type — text/plain alone is too generic
  return types.includes(MIME);
}
