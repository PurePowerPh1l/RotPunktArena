import { invokeAdmin } from "./invokeAdmin";
import { invoke } from "@tauri-apps/api/core";

export async function listTrainingHistory(
  limit?: number,
  filter?: { personId?: string | null; shooterName?: string | null },
  offset = 0,
): Promise<import("@rotpunktarena/domain").TrainingSessionSummary[]> {
  return invoke("list_training_history", {
    limit: limit ?? null,
    offset,
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
  return invokeAdmin("clear_training_history", {
    personId: filter?.personId ?? null,
    shooterName: filter?.shooterName ?? null,
  });
}

export async function promoteTrainingShooter(
  shooterName: string,
): Promise<import("@rotpunktarena/domain").PromoteTrainingShooterResult> {
  return invokeAdmin("promote_training_shooter", { shooterName });
}

export type TrainingLifetime = { key: string; sessionCount: number; shotCount: number; pointsTotal: number; sr: number; previousSr: number; previousPointsTotal: number; previousShotCount: number };
export function getTrainingLifetime(): Promise<TrainingLifetime[]> { return invoke("get_training_lifetime"); }
