import type { Competition, EntryStatus } from "@rotpunktarena/domain";

export const ENTRY_LABEL: Record<EntryStatus, string> = {
  waiting: "Wartend",
  probe: "Probe",
  active: "Aktiv",
  done: "Fertig",
};

export const STATUS_LABEL = {
  draft: "Entwurf",
  active: "Aktiv",
  closed: "Geschlossen",
  archived: "Archiviert",
  template: "Vorlage",
} as const;

export const KIND_LABEL = {
  competition: "Wettkampf",
  training: "Trainingswettkampf",
} as const;

export const SCORING_LABEL = {
  ringe: "Punkte",
  teiler: "Teiler",
} as const;

/** Compact meta line for competition lists in Verwaltung. */
export function competitionListMeta(
  c: Competition,
  opts?: { omitStatus?: boolean },
): string {
  if (c.status === "archived" && !opts?.omitStatus) {
    return `${c.date} · ${STATUS_LABEL.archived}`;
  }
  const bits = [
    c.date,
    opts?.omitStatus
      ? null
      : (STATUS_LABEL[c.status as keyof typeof STATUS_LABEL] ?? c.status),
    c.discipline?.trim() || null,
    `${c.maxShots} Schüsse`,
    SCORING_LABEL[c.scoringMode === "teiler" ? "teiler" : "ringe"],
    c.kind === "training" ? KIND_LABEL.training : null,
    c.nachkaufEnabled ? "Nachkauf" : null,
    c.teamScoringEnabled ? "Teams" : null,
  ];
  return bits.filter(Boolean).join(" · ");
}
