import { invoke } from "@tauri-apps/api/core";
import { requireAdminAuth } from "../access/requireAdminAuth";

/** User flow only; every corresponding Rust command independently authorizes. */
export async function invokeAdmin<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!(await requireAdminAuth())) throw new Error("Verwaltungsaktion abgebrochen — Admin-Freigabe erforderlich.");
  return invoke<T>(command, args);
}
