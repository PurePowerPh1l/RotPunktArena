/**
 * Last-wins save queue for full UiPrefs writes.
 * At most one request in flight; newer requests replace `pending`.
 */

export type SaveQueueState<T> = {
  inflight: boolean;
  pending: T | null;
};

export function emptySaveQueue<T>(): SaveQueueState<T> {
  return { inflight: false, pending: null };
}

/** Enqueue or start a save. `start` is set only when a new request should begin. */
export function onSaveRequested<T>(
  state: SaveQueueState<T>,
  next: T,
): { state: SaveQueueState<T>; start: T | null } {
  if (state.inflight) {
    return { state: { inflight: true, pending: next }, start: null };
  }
  return { state: { inflight: true, pending: null }, start: next };
}

/**
 * After a successful write: continue with the latest pending snapshot, or idle.
 */
export function onSaveFinished<T>(
  state: SaveQueueState<T>,
): { state: SaveQueueState<T>; continueWith: T | null } {
  if (state.pending != null) {
    return {
      state: { inflight: true, pending: null },
      continueWith: state.pending,
    };
  }
  return {
    state: { inflight: false, pending: null },
    continueWith: null,
  };
}

/** On failure: drop pending and clear inflight (caller restores confirmed prefs). */
export function onSaveFailed<T>(): SaveQueueState<T> {
  return emptySaveQueue();
}
