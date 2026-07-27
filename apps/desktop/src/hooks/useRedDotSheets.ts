/**
 * Setup / Wake sheet orchestration for Arena (Lab ≠ product Owner policy).
 * Owns open/dismiss state and effects only — sheet JSX stays in the view.
 */
import { useEffect, useState } from "react";

export type RedDotSheetsLink = {
  rfcommFeature: boolean;
  linked: boolean;
  needsSetup: boolean;
  hasTarget: boolean;
  rfcommStatus: string;
  refresh: () => void | Promise<void>;
};

type Args = {
  link: RedDotSheetsLink;
  /** Bumped from top-bar badge to (re)open first-setup sheet. */
  setupRequestNonce?: number;
};

export function useRedDotSheets({ link, setupRequestNonce = 0 }: Args) {
  const [setupOpen, setSetupOpen] = useState(false);
  const [setupDismissed, setSetupDismissed] = useState(false);
  const [wakeOpen, setWakeOpen] = useState(false);
  const [wakeDismissed, setWakeDismissed] = useState(false);

  // First setup sheet: open when no known target (unless dismissed).
  useEffect(() => {
    if (!link.rfcommFeature) return;
    if (link.linked) {
      setSetupOpen(false);
      setSetupDismissed(false);
      setWakeOpen(false);
      setWakeDismissed(false);
      return;
    }
    if (link.needsSetup && !setupDismissed) {
      setSetupOpen(true);
      setWakeOpen(false);
    }
  }, [link.rfcommFeature, link.linked, link.needsSetup, setupDismissed]);

  // Known target, idle, not linked → small wake hint (unless setup / dismissed / busy).
  useEffect(() => {
    if (!link.rfcommFeature || link.needsSetup || link.linked || setupOpen) {
      return;
    }
    const busy =
      link.rfcommStatus === "connecting" ||
      link.rfcommStatus === "discovering" ||
      link.rfcommStatus === "reconnecting";
    if (busy) {
      setWakeOpen(false);
      return;
    }
    if (
      link.hasTarget &&
      !wakeDismissed &&
      (link.rfcommStatus === "idle" ||
        link.rfcommStatus === "needsPairing" ||
        link.rfcommStatus === "faulted")
    ) {
      setWakeOpen(true);
    }
  }, [
    link.rfcommFeature,
    link.needsSetup,
    link.linked,
    link.hasTarget,
    link.rfcommStatus,
    setupOpen,
    wakeDismissed,
  ]);

  // Badge / CTA: force-open sheet even after „Später“.
  useEffect(() => {
    if (setupRequestNonce < 1) return;
    setSetupDismissed(false);
    setSetupOpen(true);
    setWakeOpen(false);
  }, [setupRequestNonce]);

  const closeSetup = () => {
    setSetupOpen(false);
    setSetupDismissed(true);
  };

  const linkedSetup = () => {
    void link.refresh();
    setSetupOpen(false);
    setSetupDismissed(false);
  };

  const closeWake = () => {
    setWakeOpen(false);
    setWakeDismissed(true);
  };

  const linkedWake = () => {
    void link.refresh();
    setWakeOpen(false);
    setWakeDismissed(false);
  };

  const reopenSetup = () => {
    setSetupDismissed(false);
    setSetupOpen(true);
  };

  const reopenWake = () => {
    setWakeDismissed(false);
    setWakeOpen(true);
  };

  return {
    setupOpen,
    wakeSheetOpen: wakeOpen && !setupOpen,
    showSetupReopen: link.needsSetup && setupDismissed && !setupOpen,
    showWakeReopen:
      !link.needsSetup &&
      link.hasTarget &&
      !link.linked &&
      wakeDismissed &&
      !wakeOpen &&
      !setupOpen,
    closeSetup,
    linkedSetup,
    closeWake,
    linkedWake,
    reopenSetup,
    reopenWake,
  };
}
