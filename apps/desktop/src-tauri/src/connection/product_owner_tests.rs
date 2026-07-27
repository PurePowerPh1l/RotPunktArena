//! Characterize product Owner transitions (Startup Nuclear contract).
//!
//! No Windows FFI — models the decisions in `owner::handle_cmd` /
//! `after_known_ready` / `run_nuclear` fail & cancel outcomes so refactors
//! cannot silently change the hardware-validated policy.
//!
//! Product baseline: commit `5b1dcd3` / merge `47524dd`.
//!
//! Fake vs Real: this Fake must match Owner **decision** outcomes. It does not
//! run Winsock / auth_hook / persist IO — those are named in comments where
//! Real diverges (e.g. persist Err → Faulted is modeled as an outcome hook).

use super::connect_policy::ConnectOrigin;
use super::nuclear::ForgetScope;
use super::status::ConnectionStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FakeCmd {
    Start,
    NuclearLink,
    PauseForSetup,
    CancelConnect,
    ForgetTarget,
    SelectTargetPersistOk,
    SelectTargetPersistFail,
    Shutdown,
}

/// Nuclear flight interrupt while Connecting (mirrors `run_nuclear` cmd pump).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NuclearInterrupt {
    CancelConnect,
    PauseForSetup,
    ForgetTarget,
}

/// Minimal product Owner: Known Target gate, single-flight Connecting,
/// Startup/Badge Nuclear outcomes, Idle after Startup fail or link-lost,
/// NeedsPairing after Badge/Setup fail.
#[derive(Debug)]
struct ProductOwner {
    generation: u64,
    status: ConnectionStatus,
    has_target: bool,
    skip_startup_nuclear: bool,
    /// Forget scope last Nuclear would use (PrimaryOnly vs AllHints).
    last_forget_primary_only: Option<bool>,
    last_origin: Option<ConnectOrigin>,
    nuclear_starts: u32,
}

impl ProductOwner {
    fn new(has_target: bool) -> Self {
        Self {
            generation: 0,
            status: ConnectionStatus::Idle,
            has_target,
            skip_startup_nuclear: false,
            last_forget_primary_only: None,
            last_origin: None,
            nuclear_starts: 0,
        }
    }

    fn bump(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn begin_nuclear(&mut self, origin: ConnectOrigin) {
        self.bump();
        self.status = ConnectionStatus::Connecting;
        self.last_origin = Some(origin);
        let primary_only = matches!(origin, ConnectOrigin::StartupAuto);
        self.last_forget_primary_only = Some(primary_only);
        self.nuclear_starts += 1;
    }

    /// Mirrors Owner: drop Start/Nuclear while Connecting (outer handle_cmd).
    fn handle(&mut self, cmd: FakeCmd) -> bool {
        match cmd {
            FakeCmd::Shutdown => {
                self.bump();
                true
            }
            FakeCmd::Start => {
                if self.status == ConnectionStatus::Connecting {
                    return false;
                }
                if !self.has_target {
                    self.status = ConnectionStatus::NeedsTarget;
                    return false;
                }
                if self.skip_startup_nuclear {
                    self.status = ConnectionStatus::Idle;
                    return false;
                }
                self.begin_nuclear(ConnectOrigin::StartupAuto);
                false
            }
            FakeCmd::NuclearLink => {
                if self.status == ConnectionStatus::Connecting {
                    return false;
                }
                if !self.has_target {
                    self.status = ConnectionStatus::NeedsTarget;
                    return false;
                }
                self.begin_nuclear(ConnectOrigin::BadgeNuclear);
                false
            }
            FakeCmd::PauseForSetup => {
                // Outer handle_cmd (not inside nuclear pump): bump + Discovering.
                self.bump();
                self.status = ConnectionStatus::Discovering;
                false
            }
            FakeCmd::CancelConnect => {
                self.bump();
                self.status = ConnectionStatus::Idle;
                false
            }
            FakeCmd::ForgetTarget => {
                self.bump();
                self.has_target = false;
                self.status = ConnectionStatus::NeedsTarget;
                false
            }
            FakeCmd::SelectTargetPersistOk => {
                self.bump();
                self.has_target = true;
                self.status = ConnectionStatus::Idle;
                false
            }
            FakeCmd::SelectTargetPersistFail => {
                // Real: save_known_target Err → Faulted (`owner.rs` SelectTarget).
                self.bump();
                self.status = ConnectionStatus::Faulted;
                false
            }
        }
    }

    /// Interrupt while Nuclear worker runs (`run_nuclear` pump): bump only;
    /// when result arrives with stale gen → Idle (not Discovering / NeedsTarget).
    /// Real Forget during pump does **not** clear target in the pump arm.
    fn interrupt_nuclear_flight(&mut self, kind: NuclearInterrupt) {
        assert_eq!(self.status, ConnectionStatus::Connecting);
        let started = self.generation;
        self.bump();
        assert!(self.generation > started);
        self.status = ConnectionStatus::Idle;
        self.last_origin = None;
        match kind {
            NuclearInterrupt::ForgetTarget => {
                // Characterized Real: pump only bumps — target remains until full Forget.
            }
            NuclearInterrupt::CancelConnect | NuclearInterrupt::PauseForSetup => {}
        }
    }

    fn nuclear_ok(&mut self, started_gen: u64) {
        if started_gen != self.generation {
            return;
        }
        self.status = ConnectionStatus::Linked;
    }

    fn nuclear_ok_but_persist_fail(&mut self, started_gen: u64) {
        if started_gen != self.generation {
            return;
        }
        self.status = ConnectionStatus::Faulted;
    }

    fn nuclear_fail(&mut self, started_gen: u64) {
        if started_gen != self.generation {
            return;
        }
        let origin = self.last_origin.unwrap_or(ConnectOrigin::StartupAuto);
        self.status = match origin {
            ConnectOrigin::StartupAuto => ConnectionStatus::Idle,
            ConnectOrigin::BadgeNuclear | ConnectOrigin::SetupNuclear => {
                ConnectionStatus::NeedsPairing
            }
            ConnectOrigin::None => ConnectionStatus::Idle,
        };
        self.last_origin = None;
    }

    fn link_lost(&mut self) {
        self.status = ConnectionStatus::Idle;
    }
}

fn forget_scope_for_origin(origin: ConnectOrigin) -> ForgetScope {
    match origin {
        ConnectOrigin::StartupAuto => ForgetScope::PrimaryOnly,
        _ => ForgetScope::AllRedDotHints,
    }
}

#[test]
fn start_without_target_needs_target() {
    let mut o = ProductOwner::new(false);
    assert!(!o.handle(FakeCmd::Start));
    assert_eq!(o.status, ConnectionStatus::NeedsTarget);
    assert_eq!(o.nuclear_starts, 0);
}

#[test]
fn start_known_begins_startup_nuclear_primary_only() {
    let mut o = ProductOwner::new(true);
    assert!(!o.handle(FakeCmd::Start));
    assert_eq!(o.status, ConnectionStatus::Connecting);
    assert_eq!(o.last_forget_primary_only, Some(true));
    assert_eq!(o.last_origin, Some(ConnectOrigin::StartupAuto));
    assert_eq!(o.nuclear_starts, 1);
    assert_eq!(
        forget_scope_for_origin(ConnectOrigin::StartupAuto),
        ForgetScope::PrimaryOnly
    );
}

#[test]
fn start_skip_env_stays_idle() {
    let mut o = ProductOwner::new(true);
    o.skip_startup_nuclear = true;
    assert!(!o.handle(FakeCmd::Start));
    assert_eq!(o.status, ConnectionStatus::Idle);
    assert_eq!(o.nuclear_starts, 0);
}

#[test]
fn badge_nuclear_while_connecting_is_single_flight() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    let gen = o.generation;
    assert!(!o.handle(FakeCmd::NuclearLink));
    assert!(!o.handle(FakeCmd::Start));
    assert_eq!(o.generation, gen);
    assert_eq!(o.nuclear_starts, 1);
    assert_eq!(o.status, ConnectionStatus::Connecting);
}

#[test]
fn startup_nuclear_fail_goes_idle_not_retry() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    let gen = o.generation;
    o.nuclear_fail(gen);
    assert_eq!(o.status, ConnectionStatus::Idle);
    assert_eq!(o.nuclear_starts, 1);
}

#[test]
fn badge_nuclear_fail_goes_needs_pairing() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::NuclearLink);
    assert_eq!(o.last_forget_primary_only, Some(false));
    assert_eq!(
        forget_scope_for_origin(ConnectOrigin::BadgeNuclear),
        ForgetScope::AllRedDotHints
    );
    let gen = o.generation;
    o.nuclear_fail(gen);
    assert_eq!(o.status, ConnectionStatus::NeedsPairing);
}

#[test]
fn setup_nuclear_fail_goes_needs_pairing() {
    let mut o = ProductOwner::new(true);
    o.begin_nuclear(ConnectOrigin::SetupNuclear);
    let gen = o.generation;
    o.nuclear_fail(gen);
    assert_eq!(o.status, ConnectionStatus::NeedsPairing);
    assert_eq!(
        forget_scope_for_origin(ConnectOrigin::SetupNuclear),
        ForgetScope::AllRedDotHints
    );
}

#[test]
fn nuclear_ok_links() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::NuclearLink);
    assert_eq!(o.last_forget_primary_only, Some(false));
    let gen = o.generation;
    o.nuclear_ok(gen);
    assert_eq!(o.status, ConnectionStatus::Linked);
}

#[test]
fn nuclear_ok_persist_fail_faulted() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::NuclearLink);
    let gen = o.generation;
    o.nuclear_ok_but_persist_fail(gen);
    assert_eq!(o.status, ConnectionStatus::Faulted);
}

#[test]
fn select_target_persist_fail_faulted() {
    let mut o = ProductOwner::new(false);
    o.handle(FakeCmd::SelectTargetPersistFail);
    assert_eq!(o.status, ConnectionStatus::Faulted);
}

#[test]
fn select_target_persist_ok_idle_with_target() {
    let mut o = ProductOwner::new(false);
    o.handle(FakeCmd::SelectTargetPersistOk);
    assert!(o.has_target);
    assert_eq!(o.status, ConnectionStatus::Idle);
}

#[test]
fn link_lost_idle_no_auto_nuclear() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    let gen = o.generation;
    o.nuclear_ok(gen);
    o.link_lost();
    assert_eq!(o.status, ConnectionStatus::Idle);
    assert_eq!(o.nuclear_starts, 1);
}

#[test]
fn cancel_during_connecting_bumps_and_idles() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    let gen0 = o.generation;
    o.handle(FakeCmd::CancelConnect);
    assert!(o.generation > gen0);
    assert_eq!(o.status, ConnectionStatus::Idle);
}

#[test]
fn pause_outside_nuclear_discovers() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::PauseForSetup);
    assert_eq!(o.status, ConnectionStatus::Discovering);
}

#[test]
fn pause_during_nuclear_flight_ends_idle_not_discovering() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    assert!(o.has_target);
    o.interrupt_nuclear_flight(NuclearInterrupt::PauseForSetup);
    assert_eq!(o.status, ConnectionStatus::Idle);
    assert!(o.has_target);
}

#[test]
fn cancel_during_nuclear_flight_ends_idle() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    o.interrupt_nuclear_flight(NuclearInterrupt::CancelConnect);
    assert_eq!(o.status, ConnectionStatus::Idle);
}

#[test]
fn forget_during_nuclear_flight_bumps_idle_keeps_target() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    o.interrupt_nuclear_flight(NuclearInterrupt::ForgetTarget);
    assert_eq!(o.status, ConnectionStatus::Idle);
    assert!(o.has_target);
}

#[test]
fn stale_generation_ignored_after_cancel() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::Start);
    let stale = o.generation;
    o.handle(FakeCmd::CancelConnect);
    o.nuclear_ok(stale);
    assert_eq!(o.status, ConnectionStatus::Idle);
    o.nuclear_fail(stale);
    assert_eq!(o.status, ConnectionStatus::Idle);
}

#[test]
fn forget_clears_target() {
    let mut o = ProductOwner::new(true);
    o.handle(FakeCmd::ForgetTarget);
    assert!(!o.has_target);
    assert_eq!(o.status, ConnectionStatus::NeedsTarget);
}

#[test]
fn shutdown_returns_exit() {
    let mut o = ProductOwner::new(true);
    assert!(o.handle(FakeCmd::Shutdown));
}

#[test]
fn connect_origin_strings_startup_and_badge() {
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
}
