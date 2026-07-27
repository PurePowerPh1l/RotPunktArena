/** Shared German formatting helpers for scores and person names. */

export function formatScoreDe(v: number): string {
  return v.toLocaleString("de-DE", {
    maximumFractionDigits: 1,
    minimumFractionDigits: Number.isInteger(v) ? 0 : 1,
  });
}

/** Compact score (no forced fractional digit). */
export function formatScoreCompact(v: number): string {
  return Number.isInteger(v) ? String(v) : v.toFixed(1);
}

export function formatPersonName(
  lastName?: string | null,
  firstName?: string | null,
  fallback = "Schütze",
): string {
  const last = (lastName ?? "").trim();
  const first = (firstName ?? "").trim();
  if (last === "—" || last === "-") {
    return first || fallback;
  }
  const label = `${last}, ${first}`.replace(/^,\s*|,\s*$/g, "").trim();
  return label || fallback;
}
