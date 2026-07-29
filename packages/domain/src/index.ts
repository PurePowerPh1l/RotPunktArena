/**
 * Shared domain types — Stand-Client + spätere Vereinsserver.
 * camelCase matches Rust serde rename_all.
 */

export type ConnectionStatus = "searching" | "connected" | "disconnected";

export type TransportKind = "simulator" | "serial" | "tcp";

export type CompetitionStatus =
  | "draft"
  | "active"
  | "closed"
  | "archived"
  | "template";

/** Official match vs. training match with start list. */
export type CompetitionKind = "competition" | "training";

export type EntryStatus = "waiting" | "probe" | "active" | "done";

export type ScoringMode = "ringe" | "teiler";

/**
 * Append-only event kinds stored in SQLite (`events.kind`).
 * Must match Rust Arena/DB writers verbatim.
 * UI emit channels (`shot`, `connection`, `series_complete`) are separate — not EventKind.
 */
export type EventKind =
  | "session_started"
  | "session_ended"
  | "shot_received"
  | "frame_parse_error"
  | "shot_rejected_limit";

export interface Person {
  id: string;
  firstName: string;
  lastName: string;
  club?: string | null;
  createdAt: string;
  /** Soft-hidden from active Büro lists / autocomplete. */
  archived?: boolean;
}

export interface Competition {
  id: string;
  name: string;
  date: string;
  discipline: string;
  maxShots: number;
  scoringMode: ScoringMode;
  status: CompetitionStatus;
  createdAt: string;
  /** Official competition (default) or training match with start list. */
  kind?: CompetitionKind;
  /** When true, shooters may start additional full series after `done`. */
  nachkaufEnabled?: boolean;
  /**
   * @deprecated Legacy DB column (extra-shot cap). Create always stores 0;
   * Nachkauf is full series restarts, not extra shots in one session.
   */
  nachkaufShots?: number;
  /** Team scoring enabled for this competition. */
  teamScoringEnabled?: boolean;
  /** How many best shooters count toward the team total. */
  teamCount?: number;
}

export interface CompetitionEntry {
  id: string;
  competitionId: string;
  personId: string;
  startOrder: number;
  status: EntryStatus;
  /** Joined display fields */
  firstName?: string;
  lastName?: string;
  club?: string | null;
  /**
   * Count of started Nachkauf series (full restarts after `done`), not extra shots.
   * Incremented by the backend on each Nachkauf start.
   */
  nachkaufPurchased?: number;
}

export interface CompetitionTeam {
  id: string;
  /** Empty for global teams; may echo competition context when listing. */
  competitionId?: string;
  name: string;
  sortOrder: number;
  archived?: boolean;
  /** Global person membership. */
  memberPersonIds?: string[];
  /** Entry IDs on the start list of the requested competition. */
  memberEntryIds: string[];
}

export interface TeamMemberScore {
  entryId: string;
  firstName?: string | null;
  lastName?: string | null;
  punkteTotal: number;
  teilerAvg: number;
  shotCount: number;
  counts: boolean;
}

export interface TeamResultSummary {
  teamId: string;
  competitionId: string;
  name: string;
  sortOrder: number;
  memberCount: number;
  countingMembers: number;
  punkteTotal: number;
  teilerSum: number;
  teilerAvg: number;
  members: TeamMemberScore[];
  /** 1-based; null if no counting members. Assigned in Rust. */
  rankPunkte?: number | null;
  /** 1-based; null if no counting members. Assigned in Rust. */
  rankTeiler?: number | null;
}

export interface SessionInfo {
  id: string;
  shooterName: string;
  startedAt: string;
  endedAt?: string | null;
  competitionId?: string | null;
  entryId?: string | null;
  personId?: string | null;
}

export interface UiShot {
  shotIndex: number;
  valueRaw: number;
  distanceRaw: number;
  x: number;
  y: number;
  valueDisplay: number;
  distanceDisplay: number;
  seriesTotal: number;
  /** Running Σ Teiler from Rust — do not recompute in UI. */
  seriesTeilerTotal: number;
}

export interface LiveState {
  status: ConnectionStatus;
  transport: TransportKind;
  port?: string | null;
  session?: SessionInfo | null;
  shots: UiShot[];
  seriesTotal: number;
  /** Running Σ Teiler from Rust — do not recompute in UI. */
  seriesTeilerTotal: number;
  lastShot?: UiShot | null;
  autoFire: boolean;
  /**
   * Legacy name: true when the desktop build includes native RFCOMM (`feature = "rfcomm"`).
   * Means hardware-link support is compiled in — not Virtual-COM / Cargo `serial`.
   */
  serialFeature: boolean;
  /** Competition shot limit; null/undefined = unlimited (endless training). */
  maxShots?: number | null;
  /** True after maxShots reached and session auto-closed. */
  seriesComplete?: boolean;
  /** Last training history save decision after stop/reset. */
  trainingSave?: TrainingSaveInfo | null;
  /** Training endless mode — no series limit, not written to history/stats. */
  endlessMode?: boolean;
}

/** Outcome of smart training auto-save (Rust is source of truth). */
export interface TrainingSaveInfo {
  saved: boolean;
  shotCount: number;
  minShots: number;
  /** saved | empty | too_short | not_training | endless */
  reason: string;
}

/** Fixed training series length. Must match Rust `TRAINING_SERIES_SHOTS`. */
export const TRAINING_SERIES_SHOTS = 10;

/** Must match Rust `TRAINING_HISTORY_MIN_SHOTS` (= full series only). */
export const TRAINING_HISTORY_MIN_SHOTS = TRAINING_SERIES_SHOTS;

/** German UI copy derived from TrainingSaveInfo / min-shots policy. */
export function trainingHistoryMinShotsHint(minShots = TRAINING_HISTORY_MIN_SHOTS): string {
  return `Nur vollständige ${minShots}er-Serien landen in der Statistik — kürzere Abbrüche werden verworfen.`;
}

export function trainingResetConfirmMessage(minShots = TRAINING_HISTORY_MIN_SHOTS): string {
  return `Aktuelle Serie beenden und neue starten?\n(${trainingHistoryMinShotsHint(minShots)})`;
}

export function trainingEndlessResetConfirmMessage(): string {
  return "Aktuelle Endlos-Serie verwerfen und neu starten?\n(Endlosmodus wird nicht in der Statistik gespeichert.)";
}

export function trainingHistoryClearConfirmMessage(scopeLabel: string): string {
  return (
    `Statistik zurücksetzen (${scopeLabel})?\n` +
    `Serien verschwinden aus der Statistik (Charts/Achievements). ` +
    `Schussdaten bleiben in der Datenbank erhalten.`
  );
}

export function trainingSaveUiMessage(info: TrainingSaveInfo): string | null {
  switch (info.reason) {
    case "saved":
      return `Serie gespeichert (${info.shotCount} Schüsse)`;
    case "too_short":
      return `Serie beendet (${info.shotCount} Schüsse) — Statistik nur bei voller ${info.minShots}er-Serie`;
    case "empty":
      return "Serie beendet — keine Schüsse, nichts gespeichert";
    case "endless":
      return `Endlosmodus beendet (${info.shotCount} Schüsse) — nicht in Statistik`;
    default:
      return null;
  }
}

export interface SeriesCompletePayload {
  maxShots: number;
  shotCount: number;
  seriesTotal: number;
  shooterName: string;
}

/** Saved training series for progress history / charts. */
export interface TrainingSessionSummary {
  id: string;
  shooterName: string;
  personId?: string | null;
  startedAt: string;
  endedAt: string;
  shotCount: number;
  punkteTotal: number;
  teilerSum: number;
  teilerAvg: number;
}

/** Saved training series with shot marks for Statistik detail. */
export interface TrainingSessionDetail {
  summary: TrainingSessionSummary;
  shots: UiShot[];
}

export interface TrainingShooterOption {
  personId?: string | null;
  shooterName: string;
  sessionCount: number;
}

/** Freitext-Schütze → Büro-Person (+ optional Session-Verknüpfung). */
export interface PromoteTrainingShooterResult {
  person: Person;
  created: boolean;
  linkedSessions: number;
}

export interface EntryResultSummary {
  entryId: string;
  competitionId: string;
  personId: string;
  startOrder: number;
  status: EntryStatus;
  firstName?: string | null;
  lastName?: string | null;
  club?: string | null;
  /** Best series session (by scoring mode), if any. */
  sessionId?: string | null;
  sessionEndedAt?: string | null;
  shotCount: number;
  punkteTotal: number;
  teilerSum: number;
  teilerAvg: number;
  /** 1-based; null if no shots. Assigned in Rust. */
  rankPunkte?: number | null;
  /** 1-based; null if no shots. Assigned in Rust. */
  rankTeiler?: number | null;
}

/** One series (session) for an entry — chronological, with best-of flag. */
export interface SeriesResultSummary {
  sessionId: string;
  /** 1-based chronological series number (1 = Hauptrunde). */
  seriesIndex: number;
  startedAt?: string | null;
  endedAt?: string | null;
  shotCount: number;
  punkteTotal: number;
  teilerSum: number;
  teilerAvg: number;
  isBest: boolean;
  /** True when seriesIndex > 1. */
  isNachkauf?: boolean;
  /** Present on `get_entry_result`; empty on `list_entry_series`. */
  shots?: UiShot[];
}

export interface EntryResultDetail {
  /** Aggregates from the best series (by competition scoring mode). */
  summary: EntryResultSummary;
  competitionName: string;
  scoringMode: ScoringMode;
  /** Competition maxShots per series. */
  maxShots: number;
  /** Shots of the best series (compat for existing UI). */
  shots: UiShot[];
  /** All series chronologically; best marked with `isBest`. */
  series: SeriesResultSummary[];
}

export interface ConnectionUpdate {
  status: ConnectionStatus;
  transport: TransportKind;
  port?: string | null;
  detail?: string | null;
}

export interface ShotEventPayload {
  valueRaw: number;
  distanceRaw: number;
  x: number;
  y: number;
  valueDisplay: number;
  distanceDisplay: number;
  shotIndex: number;
}

export interface ConnectionEventPayload {
  status: ConnectionStatus;
  transport: TransportKind;
  port?: string;
  detail?: string;
}

export interface DomainEvent {
  id: string;
  sessionId: string;
  kind: EventKind;
  createdAt: string;
  payload: ShotEventPayload | ConnectionEventPayload | Record<string, unknown>;
}

/** Interrupted / recoverable session — matches Rust `RecoverySessionInfo`. */
export interface RecoverySessionInfo {
  id: string;
  shooterName: string;
  startedAt: string;
  competitionId?: string | null;
  entryId?: string | null;
  personId?: string | null;
  shotCount: number;
  lastAutosaveSequence?: number | null;
  lastAutosaveAt?: string | null;
  recoveryState?: string;
}

/** Developer diagnostics snapshot — matches Rust `DevDiagnostics`. */
export interface DevDiagnostics {
  dbPath: string;
  schemaVersion: number;
  hasCompetitionId: boolean;
  hasEntryId: boolean;
  parserVersion: string;
  sessionId: string | null;
  sessionShots: number;
  totalShots: number;
  totalFrames: number;
  shotReceivedEvents: number;
  uncleanSessions: string[];
  recentShots: Array<{
    shotIndex: number;
    score: number;
    x: number;
    y: number;
    sessionSequence: number;
    frameId: string;
    createdAt: string;
  }>;
  liveRunning: boolean;
  liveUiShots: number;
}

/** @deprecated Prefer Person */
export interface Shooter {
  id: string;
  name: string;
}

/** @deprecated Prefer SessionInfo */
export type TrainingSession = SessionInfo;

/** Mirrors Rust `UiPrefs` / settings key `ui.prefs` (camelCase). */
export type AppViewPref = "live" | "history" | "bureau";
export type ColorSchemePref = "system" | "light" | "dark";
export type ScoreDisplayPref = "punkte" | "teiler";
export type HitFeedbackPref = "normal" | "reduced" | "minimal";
export type TargetFitPref = "auto" | "calm" | "aggressive";

export interface UiPrefs {
  startView: AppViewPref;
  rememberLastView: boolean;
  lastView: AppViewPref | null;
  compactUi: boolean;
  largeText: boolean;
  colorScheme: ColorSchemePref;
  reducedMotion: boolean;
  scoreDisplay: ScoreDisplayPref;
  rememberScoreDisplay: boolean;
  hitFeedback: HitFeedbackPref;
  targetFit: TargetFitPref;
}

/**
 * Defensive FE load placeholder only — authoritative defaults live in
 * Rust `UiPrefs::default()`. Keep values in sync via settings.selftest.
 */
export const UI_PREFS_LOAD_PLACEHOLDER: UiPrefs = {
  startView: "live",
  rememberLastView: false,
  lastView: null,
  compactUi: false,
  largeText: false,
  colorScheme: "system",
  reducedMotion: false,
  scoreDisplay: "punkte",
  rememberScoreDisplay: false,
  hitFeedback: "normal",
  targetFit: "auto",
};
