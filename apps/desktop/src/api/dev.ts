import { invoke } from "@tauri-apps/api/core";
import type { DevDiagnostics, LiveState } from "@rotpunktarena/domain";

export type { DevDiagnostics };

export async function devDiagnostics(): Promise<DevDiagnostics> {
  return invoke("dev_diagnostics");
}

export async function devInjectTestShot(x?: number, y?: number): Promise<LiveState> {
  return invoke("dev_inject_test_shot", { x: x ?? null, y: y ?? null });
}
