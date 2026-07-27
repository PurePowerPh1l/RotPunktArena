//! Shared timing constants for the RFCOMM owner thread (ENQ / read slice).
//! Soft page timeouts live next to Soft autoconnect in `owner.rs`.

use std::time::Duration;

/// Give the remote a moment after socket connect before hammering ENQ.
pub const POST_CONNECT_SETTLE: Duration = Duration::from_millis(800);
pub const ENQ_INTERVAL: Duration = Duration::from_millis(500);
pub const ENQ_WRITE_TIMEOUT: Duration = Duration::from_millis(3000);
pub const READ_SLICE: Duration = Duration::from_millis(50);
pub const CMD_IDLE: Duration = Duration::from_millis(50);
/// Tolerate a few flaky ENQ/read failures before tearing the link down.
pub const IO_FAIL_LIMIT: u32 = 6;
