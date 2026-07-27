import { invoke } from "@tauri-apps/api/core";
import { assertCapability, getAppAccessSnapshot } from "../access";

export type DbBackupInfo = {
  name: string;
  path: string;
  sizeBytes: number;
  modifiedAt?: string | null;
};

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
