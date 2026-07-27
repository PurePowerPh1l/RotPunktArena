import type { ColorSchemePref } from "@rotpunktarena/domain";

/** Resolve stored pref + optional system dark flag → concrete scheme. */
export function resolveEffectiveColorScheme(
  colorScheme: ColorSchemePref,
  systemPrefersDark: boolean,
): "light" | "dark" {
  if (colorScheme === "light") return "light";
  if (colorScheme === "dark") return "dark";
  return systemPrefersDark ? "dark" : "light";
}
