//! Idle wait decision for the poll loop (product, not diagnose).
//!
//! The shared poll calls `SimulatorControl::wait_timeout(80ms)` as an idle
//! seam. On RFCOMM that wait must not run while a real incomplete STX shot
//! frame is already held in the parser — proven cause of the ~90ms cluster.

/// Whether the poll loop should call `sim_control.wait_timeout(80ms)`.
///
/// - `sim_pending != 0`: never wait (existing simulator wake path).
/// - `!use_sim && incomplete_shot_frame`: skip idle wait (RFCOMM fragment).
/// - otherwise: wait as before.
pub(super) fn should_call_idle_wait(
    sim_pending: usize,
    use_sim: bool,
    incomplete_shot_frame: bool,
) -> bool {
    if sim_pending != 0 {
        return false;
    }
    if !use_sim && incomplete_shot_frame {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_pending_skips_wait() {
        assert!(!should_call_idle_wait(1, true, false));
        assert!(!should_call_idle_wait(2, false, true));
    }

    #[test]
    fn simulator_idle_still_waits() {
        assert!(should_call_idle_wait(0, true, false));
        // Incomplete flag must not affect simulator.
        assert!(should_call_idle_wait(0, true, true));
    }

    #[test]
    fn rfcomm_idle_waits_when_no_incomplete() {
        assert!(should_call_idle_wait(0, false, false));
    }

    #[test]
    fn rfcomm_skips_wait_while_incomplete() {
        assert!(!should_call_idle_wait(0, false, true));
    }
}
