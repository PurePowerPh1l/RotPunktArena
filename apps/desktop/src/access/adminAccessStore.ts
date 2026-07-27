import type { AdminAccessState } from "./types.ts";
import * as adminAuthApi from "../api/adminAuth.ts";

export type AdminAccessListener = () => void;

/**
 * Admin access: persisted credential (configured) + ephemeral session unlock.
 * Real password verify/setup go through Rust; session never persists across restart.
 */
export interface AdminAccessController {
  readonly state: AdminAccessState;
  readonly isAdminModeEnabled: boolean;
  readonly isConfigured: boolean;
  /** Load configured flag from DB; clears session unlock. */
  hydrate(): Promise<void>;
  /** First-time password setup + session unlock. */
  setupPassword(password: string): Promise<void>;
  /** Unlock with password when already configured. */
  unlock(password: string): Promise<boolean>;
  lock(): void;
  /** DEV/TEST ONLY — temporary unlock without password. */
  enableAdminForTests(): void;
}

class AdminAccessStoreImpl implements AdminAccessController {
  private configured = false;
  private sessionUnlocked = false;
  private readonly listeners = new Set<AdminAccessListener>();

  get isConfigured(): boolean {
    return this.configured;
  }

  get state(): AdminAccessState {
    if (this.sessionUnlocked) return "unlocked";
    if (this.configured) return "locked";
    return "unconfigured";
  }

  get isAdminModeEnabled(): boolean {
    return this.state === "unlocked";
  }

  async hydrate(): Promise<void> {
    const status = await adminAuthApi.getAdminAuthStatus();
    this.configured = status.configured;
    this.sessionUnlocked = false;
    this.emit();
  }

  async setupPassword(password: string): Promise<void> {
    await adminAuthApi.setupAdminPassword(password);
    this.configured = true;
    this.sessionUnlocked = true;
    this.emit();
  }

  async unlock(password: string): Promise<boolean> {
    const ok = await adminAuthApi.verifyAdminPassword(password);
    if (ok) {
      this.sessionUnlocked = true;
      this.emit();
    }
    return ok;
  }

  lock(): void {
    this.sessionUnlocked = false;
    this.emit();
  }

  /** DEV/TEST ONLY — does not persist credentials. */
  enableAdminForTests(): void {
    this.sessionUnlocked = true;
    this.emit();
  }

  subscribe(listener: AdminAccessListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Snapshot for useSyncExternalStore. */
  getSnapshot = (): AdminAccessState => this.state;

  private emit(): void {
    for (const listener of this.listeners) listener();
  }
}

/** Process-wide singleton — React UI and action handlers share one truth. */
export const adminAccessStore = new AdminAccessStoreImpl();
