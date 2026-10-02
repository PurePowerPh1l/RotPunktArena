import { invoke } from "@tauri-apps/api/core";
import { assertCapability, getAppAccessSnapshot } from "../access";

export type DbBackupInfo = {
  name: string;
  path: string;
  sizeBytes: number;
  modifiedAt?: string | null;
};

export type BackupHealth = {
  snapshot: { completed: number; queueDrops: number; lastCompletedAt: string | null; lastError: string | null };
  storageBytes: number;
};
export async function getBackupHealth(): Promise<BackupHealth> { return invoke("get_backup_health"); }
export async function retrySnapshot(): Promise<void> { return invoke("retry_snapshot"); }

/** Always allowed — no admin unlock required. */
export async function createDbBackup(): Promise<DbBackupInfo> {
  assertCapability("backup:create", getAppAccessSnapshot());
  return invoke("create_db_backup");
}

export async function listDbBackups(): Promise<DbBackupInfo[]> {
  return invoke("list_db_backups");
}

/** Requires admin unlock — enforced here, not only in the Settings UI. */
export async function restoreDbBackup(name: string): Promise<string> {
  assertCapability("backup:restore", getAppAccessSnapshot());
  return invoke("restore_db_backup", { name });
}

/** Requires admin unlock — enforced here, not only in the Settings UI. */
export async function resetAllDatabase(): Promise<void> {
  assertCapability("admin:reset", getAppAccessSnapshot());
  await invoke("reset_all_database");
}

export async function exportPersonalBackup(goals: string): Promise<{ path: string }> {
  return invoke("export_personal_backup", { goals });
}
