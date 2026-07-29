//! Domain string constants — must match `packages/domain` EventKind / status unions.

pub mod event_kind {
    pub const SESSION_STARTED: &str = "session_started";
    pub const SESSION_ENDED: &str = "session_ended";
    pub const SHOT_RECEIVED: &str = "shot_received";
    pub const FRAME_PARSE_ERROR: &str = "frame_parse_error";
    pub const SHOT_REJECTED_LIMIT: &str = "shot_rejected_limit";
    pub const PROBE_FINISHED: &str = "probe_finished";
}

/// `sessions.phase` — probe (Probeschüsse, unscored) before the scored series.
pub mod session_phase {
    pub const PROBE: &str = "probe";
    pub const MATCH: &str = "match";
}

/// `shots.classification` values.
pub mod shot_classification {
    pub const SCORED: &str = "scored";
    pub const PROBE: &str = "probe";
}

pub mod competition_status {
    pub const DRAFT: &str = "draft";
    pub const ACTIVE: &str = "active";
    pub const CLOSED: &str = "closed";
    pub const ARCHIVED: &str = "archived";
    pub const TEMPLATE: &str = "template";
}

pub mod competition_kind {
    pub const COMPETITION: &str = "competition";
    pub const TRAINING: &str = "training";
}

pub mod entry_status {
    pub const WAITING: &str = "waiting";
    pub const PROBE: &str = "probe";
    pub const ACTIVE: &str = "active";
    pub const DONE: &str = "done";
}
