import { invoke } from "@tauri-apps/api/core";
import type { LiveState, RecoverySessionInfo } from "@rotpunktarena/domain";

export type { RecoverySessionInfo };

export async function listRecoverySessions(): Promise<RecoverySessionInfo[]> {
  return invoke("list_recovery_sessions");
}

export async function closeInterruptedSession(
  sessionId: string,
): Promise<LiveState> {
  return invoke("close_interrupted_session", { sessionId });
}

export async function resumeSession(
  sessionId: string,
  useSimulator: boolean,
): Promise<LiveState> {
  return invoke("resume_session", { sessionId, useSimulator });
}

export type EmergencyExportResult = {
  path: string;
  uncleanSessionIds: string[];
  schemaVersion: number;
};

/**
 * Export the diagnostics bundle. `fileName` only chooses the file name inside
 * the app's `exports` directory — arbitrary paths are rejected by the backend.
 */
export async function exportDiagnostics(
  fileName?: string | null,
): Promise<EmergencyExportResult> {
  return invoke("export_diagnostics", { fileName: fileName ?? null });
}
