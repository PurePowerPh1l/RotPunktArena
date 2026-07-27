import type { UiShot } from "@rotpunktarena/domain";
import { tenStreak } from "../../live/presenceContract";

type Props = {
  shots: UiShot[];
  visible: boolean;
};

/** Training-only streak chip under the face. */
export function StreakChip({ shots, visible }: Props) {
  if (!visible) return null;
  const n = tenStreak(shots);
  if (n < 2) return null;

  return (
    <p className="streak-chip" role="status">
      {n}× Zehn in Folge
    </p>
  );
}
