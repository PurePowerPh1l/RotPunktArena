//! Connect policy: BondLookup, ConnectOrigin, ConnectPhase (product).
//!
//! Product Start = Startup Nuclear (PrimaryOnly) for Known BD_ADDR — see `owner.rs`.
//! Soft-Wake disposition / Soft-gate helpers were removed (retired; see
//! `docs/architecture/soft-wake-caller-matrix.md`).

use std::fmt::Display;

/// Diag / status phase (`rfcomm_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectPhase {
    #[default]
    Idle,
    /// RFCOMM paging (startup Nuclear or badge/setup Nuclear).
    Paging,
    /// Reserved wire string; product Owner does not enter Soft-Wake backoff.
    Backoff,
    /// WSAEACCES / nuclear fail stop — needs pairing UI (never auto-pair).
    AuthStop,
}

impl ConnectPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Paging => "paging",
            Self::Backoff => "backoff",
            Self::AuthStop => "authStop",
        }
    }
}

/// Who initiated the current connect attempt (for status / diag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectOrigin {
    #[default]
    None,
    /// App Start: Known BD_ADDR → exactly one Full Nuclear (Forget → Pair → RFCOMM).
    StartupAuto,
    /// Badge „Verbinden“ / repair.
    BadgeNuclear,
    /// First-setup sheet.
    SetupNuclear,
}

impl ConnectOrigin {
    pub fn as_api_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::StartupAuto => Some("startupNuclear"),
            Self::BadgeNuclear => Some("badgeNuclear"),
            Self::SetupNuclear => Some("setupNuclear"),
        }
    }
}

/// Result of asking Windows for bond state. Query errors are **not** NotBonded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BondLookup {
    /// Authenticated + Windows `fConnected` — diag only; Startup uses Nuclear.
    Bonded,
    /// Authenticated but not connected (idle/sleep) — diag only; Startup still Nuclear.
    BondedIdle,
    /// Device unknown or not authenticated — Setup / NeedsPairing.
    NotBonded,
    /// Stack/query glitch — Idle + diagnose; never connect, never open Setup.
    Unknown(String),
}

impl Default for BondLookup {
    fn default() -> Self {
        Self::NotBonded
    }
}

impl BondLookup {
    pub fn from_bond_result<E: Display>(
        r: Result<Option<crate::transport::rfcomm::discovery::BondState>, E>,
    ) -> Self {
        match r {
            Ok(Some(b)) if b.authenticated && b.connected => Self::Bonded,
            Ok(Some(b)) if b.authenticated => Self::BondedIdle,
            Ok(_) => Self::NotBonded,
            Err(e) => Self::Unknown(e.to_string()),
        }
    }

    /// Enough to treat as a known paired RedDot (JSON / pick), even if asleep.
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Bonded | Self::BondedIdle)
    }
}

/// First-setup sheet only when we **know** pairing is missing — never on Unknown.
pub fn needs_pairing_ui(bond: &BondLookup) -> bool {
    matches!(bond, BondLookup::NotBonded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::rfcomm::discovery::BondState;

    #[test]
    fn bond_lookup_tri_state() {
        assert_eq!(
            BondLookup::from_bond_result::<&str>(Ok(Some(BondState {
                authenticated: true,
                connected: true,
                ..BondState::default()
            }))),
            BondLookup::Bonded
        );
        assert_eq!(
            BondLookup::from_bond_result::<&str>(Ok(Some(BondState {
                authenticated: true,
                connected: false,
                ..BondState::default()
            }))),
            BondLookup::BondedIdle
        );
        assert_eq!(
            BondLookup::from_bond_result::<&str>(Ok(Some(BondState::default()))),
            BondLookup::NotBonded
        );
        assert_eq!(
            BondLookup::from_bond_result::<&str>(Ok(None)),
            BondLookup::NotBonded
        );
        assert_eq!(
            BondLookup::from_bond_result::<&str>(Err("glitch")),
            BondLookup::Unknown("glitch".into())
        );
        assert!(needs_pairing_ui(&BondLookup::NotBonded));
        assert!(!needs_pairing_ui(&BondLookup::Unknown("x".into())));
        assert!(!needs_pairing_ui(&BondLookup::Bonded));
        assert!(!needs_pairing_ui(&BondLookup::BondedIdle));
        assert!(BondLookup::BondedIdle.is_authenticated());
        assert!(!BondLookup::NotBonded.is_authenticated());
    }

    #[test]
    fn connect_origin_api_strings() {
        assert_eq!(
            ConnectOrigin::StartupAuto.as_api_str(),
            Some("startupNuclear")
        );
        assert_eq!(
            ConnectOrigin::BadgeNuclear.as_api_str(),
            Some("badgeNuclear")
        );
        assert_eq!(
            ConnectOrigin::SetupNuclear.as_api_str(),
            Some("setupNuclear")
        );
        assert_eq!(ConnectOrigin::None.as_api_str(), None);
    }

    #[test]
    fn connect_phase_wire_strings() {
        assert_eq!(ConnectPhase::Idle.as_str(), "idle");
        assert_eq!(ConnectPhase::Paging.as_str(), "paging");
        assert_eq!(ConnectPhase::Backoff.as_str(), "backoff");
        assert_eq!(ConnectPhase::AuthStop.as_str(), "authStop");
    }
}
