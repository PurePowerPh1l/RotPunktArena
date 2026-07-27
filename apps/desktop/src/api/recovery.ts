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
  useSimulator = true,
): Promise<LiveState> {
  return invoke("resume_session", { sessionId, useSimulator });
}

export type EmergencyExportResult = {
  path: string;
  uncleanSessionIds: string[];
  schemaVersion: number;
};

export async function exportDiagnostics(
  path?: string | null,
): Promise<EmergencyExportResult> {
  return invoke("export_diagnostics", { path: path ?? null });
}
