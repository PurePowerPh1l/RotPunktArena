/**
 * Admin auth bridge — invoke only here (and via adminAccessStore).
 * Hash/salt never leave Rust; FE only sees configured status.
 */
import { invoke } from "@tauri-apps/api/core";

export type AdminAuthStatus = {
  configured: boolean;
};

export async function getAdminAuthStatus(): Promise<AdminAuthStatus> {
  return invoke("get_admin_auth_status");
}

export async function setupAdminPassword(
  password: string,
): Promise<AdminAuthStatus> {
  return invoke("setup_admin_password", { password });
}

export async function verifyAdminPassword(password: string): Promise<boolean> {
  return invoke("verify_admin_password", { password });
}

/** Clear the server-side admin unlock (mirror of UI lock). */
export async function lockAdminSession(): Promise<void> {
  await invoke("lock_admin_session");
}

/** DEV/TEST ONLY — unlock the server-side session without a password. */
export async function devUnlockAdminSession(): Promise<void> {
  await invoke("dev_unlock_admin_session");
}
