/**
 * Applies colorScheme + reducedMotion to documentElement.
 * matchMedia is subscribed only while colorScheme === "system".
 */
import { useEffect } from "react";
import type { ColorSchemePref } from "@rotpunktarena/domain";
import { resolveEffectiveColorScheme } from "../lib/appearanceLogic";

type Args = {
  colorScheme: ColorSchemePref;
  reducedMotion: boolean;
  /** Skip DOM writes until prefs left loading. */
  enabled: boolean;
};

export function useApplyAppearance({
  colorScheme,
  reducedMotion,
  enabled,
}: Args) {
  useEffect(() => {
    if (!enabled) return;
    const root = document.documentElement;

    const applyScheme = (systemDark: boolean) => {
      root.setAttribute(
        "data-color-scheme",
        resolveEffectiveColorScheme(colorScheme, systemDark),
      );
    };

    if (colorScheme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      applyScheme(mq.matches);
      const onChange = () => applyScheme(mq.matches);
      mq.addEventListener("change", onChange);
      return () => {
        mq.removeEventListener("change", onChange);
      };
    }

    applyScheme(false);
    return undefined;
  }, [colorScheme, enabled]);

  useEffect(() => {
    if (!enabled) return;
    const root = document.documentElement;
    root.classList.toggle("is-reduced-motion", reducedMotion);
    return () => {
      root.classList.remove("is-reduced-motion");
    };
  }, [reducedMotion, enabled]);
}
