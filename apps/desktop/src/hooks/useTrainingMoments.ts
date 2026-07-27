import { useCallback, useEffect, useRef, useState } from "react";
import type { UiShot } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";
import { bestShotOf } from "./useScoreDisplay";
import { classifyShotMoment } from "../live/presenceContract";

export type MomentFlashKind = "ten" | "inner" | "best" | null;

type Args = {
  enabled: boolean;
  shots: UiShot[];
  last: UiShot | null;
  displayMode: ScoreDisplayMode;
};

/**
 * Training-only micromoments: short flash + optional toast.
 * Competition passes enabled=false.
 */
export function useTrainingMoments({ enabled, shots, last, displayMode }: Args) {
  const [flashKind, setFlashKind] = useState<MomentFlashKind>(null);
  const [toast, setToast] = useState<string | null>(null);
  const lastIndex = useRef<number | null>(null);
  const primed = useRef(false);

  const clearToast = useCallback(() => setToast(null), []);

  useEffect(() => {
    if (!enabled) {
      setFlashKind(null);
      setToast(null);
      return;
    }
    const idx = last?.shotIndex ?? null;
    if (!primed.current) {
      primed.current = true;
      lastIndex.current = idx;
      return;
    }
    if (idx == null || last == null || lastIndex.current === idx) return;
    lastIndex.current = idx;

    const kind = classifyShotMoment(last, displayMode);
    const best = bestShotOf(shots, displayMode);
    const isNewBest = best != null && best.shotIndex === last.shotIndex && shots.length > 1;

    let flash: MomentFlashKind = null;
    if (isNewBest) {
      flash = "best";
      setToast("Neue Bestmarke!");
    } else if (kind === "inner") {
      flash = "inner";
    } else if (kind === "ten") {
      flash = "ten";
    }
    if (flash) {
      setFlashKind(flash);
      const t = window.setTimeout(() => setFlashKind(null), 320);
      return () => window.clearTimeout(t);
    }
  }, [enabled, last, shots, displayMode]);

  useEffect(() => {
    if (!toast) return;
    const t = window.setTimeout(() => setToast(null), 1600);
    return () => window.clearTimeout(t);
  }, [toast]);

  useEffect(() => {
    if (!enabled) primed.current = false;
  }, [enabled]);

  return { flashKind, toast, clearToast };
}
