import type { AdminAccessState } from "./types.ts";

export type AdminAccessListener = () => void;

/**
 * In-memory admin access. Not persisted — restart ⇒ unconfigured again.
 * Real password setup/unlock are stubs for a later first-setup slice.
 */
export interface AdminAccessController {
  readonly state: AdminAccessState;
  readonly isAdminModeEnabled: boolean;
  /** DEV/TEST ONLY — temporary unlock from the Developer sheet. */
  enableAdminForTests(): void;
  lock(): void;
  /** Later: first-setup password. Not implemented. */
  setupPassword?(password: string): Promise<void>;
  /** Later: unlock with password. Not implemented. */
  unlock?(password: string): Promise<boolean>;
}

class AdminAccessStoreImpl implements AdminAccessController {
  private _state: AdminAccessState = "unconfigured";
  private readonly listeners = new Set<AdminAccessListener>();

  get state(): AdminAccessState {
    return this._state;
  }

  get isAdminModeEnabled(): boolean {
    return this._state === "unlocked";
  }

  /** DEV/TEST ONLY — does not persist across restarts. */
  enableAdminForTests(): void {
    this.setState("unlocked");
  }

  lock(): void {
    // Until real credentials exist, return to unconfigured (not a fake “locked with password”).
    this.setState("unconfigured");
  }

  subscribe(listener: AdminAccessListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Snapshot for useSyncExternalStore. */
  getSnapshot = (): AdminAccessState => this._state;

  private setState(next: AdminAccessState): void {
    if (this._state === next) return;
    this._state = next;
    for (const listener of this.listeners) listener();
  }
}

/** Process-wide singleton — React UI and action handlers share one truth. */
export const adminAccessStore = new AdminAccessStoreImpl();
