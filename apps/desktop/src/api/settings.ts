/**
 * UI prefs bridge — invoke only here (and via useUiPrefs).
 * Authoritative defaults: Rust UiPrefs::default().
 */
import { invoke } from "@tauri-apps/api/core";
import type { UiPrefs } from "@rotpunktarena/domain";

export type { UiPrefs } from "@rotpunktarena/domain";

export async function getUiPrefs(): Promise<UiPrefs> {
  return invoke("get_ui_prefs");
}

/** Full validated aggregate write — never a partial DTO. */
export async function setUiPrefs(prefs: UiPrefs): Promise<UiPrefs> {
  return invoke("set_ui_prefs", { prefs });
}
