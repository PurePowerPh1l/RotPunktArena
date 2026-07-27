import type { AppViewPref, UiPrefs } from "@rotpunktarena/domain";

/** Start tab: rememberLastView && lastView → lastView, else startView. */
export function resolveStartView(prefs: UiPrefs): AppViewPref {
  if (prefs.rememberLastView && prefs.lastView) return prefs.lastView;
  return prefs.startView;
}

/** Merge patch; turning rememberLastView off clears lastView. */
export function mergeUiPrefsPatch(
  prev: UiPrefs,
  patch: Partial<UiPrefs>,
): UiPrefs {
  const next: UiPrefs = { ...prev, ...patch };
  if (patch.rememberLastView === false) {
    next.lastView = null;
  }
  return next;
}
