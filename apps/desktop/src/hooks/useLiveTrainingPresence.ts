/**
 * Training presence glue: shot sound, score tick, ceremony cue, focus clear, rival reset.
 * Timing-sensitive — keep effect semantics 1:1 with LiveStandView (no “simplifications”).
 *
 * `clearPulseRef` must be wired to `useTrainingArenaProgress().clearPulse` during render
 * (arena progress needs `rivalEnabled` from this hook first).
 */
import { useEffect, useRef, useState } from "react";
import type { UiShot } from "@rotpunktarena/domain";
import type { ScoreDisplayMode } from "../components/TargetFace";
import {
  classifyShotMoment,
  type PlayShotOpts,
} from "../live/presenceContract";
import { bestShotOf } from "./useScoreDisplay";

type Args = {
  trainingMode: boolean;
  shots: UiShot[];
  last: UiShot | null;
  displayMode: ScoreDisplayMode;
  seriesComplete: boolean;
  canInspect: boolean;
  playShot: (opts?: PlayShotOpts) => void;
};

export function useLiveTrainingPresence({
  trainingMode,
  shots,
  last,
  displayMode,
  seriesComplete,
  canInspect,
  playShot,
}: Args) {
  const [scoreTick, setScoreTick] = useState(0);
  const [ceremonyOpen, setCeremonyOpen] = useState(false);
  const [rivalEnabled, setRivalEnabled] = useState(false);
  const [focusShot, setFocusShot] = useState<number | null>(null);
  const lastShotIndex = useRef<number | null>(null);
  const soundPrimed = useRef(false);
  const seriesDoneCue = useRef(false);
  /** Assigned by parent after `useTrainingArenaProgress` (stable clearPulse). */
  const clearPulseRef = useRef<() => void>(() => {});

  useEffect(() => {
    if (!canInspect) setFocusShot(null);
  }, [canInspect]);

  useEffect(() => {
    const idx = last?.shotIndex ?? null;
    if (!soundPrimed.current) {
      soundPrimed.current = true;
      lastShotIndex.current = idx;
      return;
    }
    if (idx == null || !last) return;
    if (lastShotIndex.current === idx) return;
    lastShotIndex.current = idx;
    if (trainingMode) {
      const kind = classifyShotMoment(last, displayMode);
      const bestNow = bestShotOf(shots, displayMode);
      const isNewBest =
        bestNow != null && bestNow.shotIndex === last.shotIndex && shots.length > 1;
      playShot({
        tier: isNewBest ? "best" : kind === "miss" ? "miss" : kind,
      });
    } else {
      playShot({ miss: last.valueDisplay <= 0 });
    }
    setScoreTick((n) => n + 1);
  }, [last, playShot, trainingMode, displayMode, shots]);

  useEffect(() => {
    if (trainingMode && seriesComplete) {
      setCeremonyOpen(true);
      if (!seriesDoneCue.current) {
        seriesDoneCue.current = true;
        playShot({ tier: "seriesDone" });
      }
    } else {
      setCeremonyOpen(false);
      seriesDoneCue.current = false;
      clearPulseRef.current();
    }
  }, [trainingMode, seriesComplete, playShot]);

  useEffect(() => {
    if (!trainingMode) setRivalEnabled(false);
  }, [trainingMode]);

  return {
    scoreTick,
    ceremonyOpen,
    rivalEnabled,
    setRivalEnabled,
    focusShot,
    setFocusShot,
    clearPulseRef,
  };
}
