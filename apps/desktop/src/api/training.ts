import { invoke } from "@tauri-apps/api/core";

export async function listTrainingHistory(
  limit?: number,
  filter?: { personId?: string | null; shooterName?: string | null },
): Promise<import("@rotpunktarena/domain").TrainingSessionSummary[]> {
  return invoke("list_training_history", {
    limit: limit ?? null,
    personId: filter?.personId ?? null,
    shooterName: filter?.shooterName ?? null,
  });
}

export async function getTrainingSessionDetail(
  sessionId: string,
): Promise<import("@rotpunktarena/domain").TrainingSessionDetail | null> {
  return invoke("get_training_session_detail", { sessionId });
}

export async function listTrainingShooters(): Promise<
  import("@rotpunktarena/domain").TrainingShooterOption[]
> {
  return invoke("list_training_shooters");
}

export async function clearTrainingHistory(filter?: {
  personId?: string | null;
  shooterName?: string | null;
}): Promise<number> {
  return invoke("clear_training_history", {
    personId: filter?.personId ?? null,
    shooterName: filter?.shooterName ?? null,
  });
}

export async function promoteTrainingShooter(
  shooterName: string,
): Promise<import("@rotpunktarena/domain").PromoteTrainingShooterResult> {
  return invoke("promote_training_shooter", { shooterName });
}
