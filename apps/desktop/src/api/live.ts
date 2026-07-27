import { invoke } from "@tauri-apps/api/core";
import type { LiveState } from "@rotpunktarena/domain";

export async function getLiveState(): Promise<LiveState> {
  return invoke("get_live_state");
}

export async function startTraining(
  shooterName: string,
  useSimulator: boolean,
  personId?: string | null,
  endless?: boolean,
): Promise<LiveState> {
  return invoke("start_training", {
    shooterName,
    useSimulator,
    personId: personId ?? null,
    endless: endless ?? false,
  });
}

export async function startEntrySession(
  entryId: string,
  useSimulator: boolean,
): Promise<LiveState> {
  return invoke("start_entry_session", { entryId, useSimulator });
}

export async function endTraining(): Promise<LiveState> {
  return invoke("end_training");
}

export async function queueSimShot(pick: {
  valueAscii: string;
  distanceAscii: string;
  xAscii: string;
  yAscii: string;
}): Promise<void> {
  await invoke("queue_sim_shot", pick);
}

export async function fireAimShot(x: number, y: number): Promise<LiveState> {
  return invoke("fire_aim_shot", { x, y });
}

export async function resetTrainingSeries(): Promise<LiveState> {
  return invoke("reset_training_series");
}

export async function setTrainingEndless(endless: boolean): Promise<LiveState> {
  return invoke("set_training_endless", { endless });
}

export async function saveTrainingSession(): Promise<LiveState> {
  return invoke("save_training_session");
}

export async function setAutoFire(on: boolean): Promise<void> {
  await invoke("set_auto_fire", { on });
}

/** Runtime repair = Nuclear (Forget→Pair→RFCOMM). Setup only via sheet. */
export async function rfcommConnectReddot(): Promise<void> {
  const st = await invoke<{
    target: unknown | null;
    status: string;
    needsSetup?: boolean;
  }>("rfcomm_status");
  if (st.status === "linked") {
    return;
  }
  if (st.needsSetup || !st.target) {
    throw new Error("RedDot noch nicht eingerichtet — bitte „Gerät einrichten“");
  }
  await invoke("rfcomm_reconnect");
}

export async function rfcommCancelConnect(): Promise<void> {
  await invoke("rfcomm_cancel_connect");
}

/** Drop persisted known target (Settings → Gerät vergessen). */
export async function rfcommForgetTarget(): Promise<void> {
  await invoke("rfcomm_forget_target");
}

export type RfcommDiagEvent = {
  ts: string;
  event: string;
  status: string;
  reason: string;
  generation?: number | null;
  addr?: string | null;
  channel?: number | null;
  winsock?: number | null;
  winsockName?: string | null;
};

export async function rfcommDiagTail(limit = 12): Promise<RfcommDiagEvent[]> {
  return invoke("rfcomm_diag_tail", { limit });
}

/** @deprecated use rfcommConnectReddot */
export async function rfcommDebugConnect(): Promise<void> {
  return rfcommConnectReddot();
}

export type SetupCandidate = {
  btAddrHex: string;
  displayName: string;
  alreadyPaired: boolean;
};

export async function rfcommSetupScan(): Promise<SetupCandidate> {
  return invoke("rfcomm_setup_scan");
}

export async function rfcommSetupConnect(
  btAddrHex: string,
  displayName?: string,
): Promise<{
  btAddrHex: string;
  displayName: string;
}> {
  return invoke("rfcomm_setup_connect", { btAddrHex, displayName });
}
