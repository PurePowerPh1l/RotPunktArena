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
