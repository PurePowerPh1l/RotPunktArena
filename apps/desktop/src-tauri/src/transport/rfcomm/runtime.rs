//! Process-wide Winsock lifecycle (exactly one WSAStartup / WSACleanup).

use super::error::TransportError;
use std::sync::atomic::{AtomicBool, Ordering};

static STARTED: AtomicBool = AtomicBool::new(false);

pub struct WinsockRuntime;

impl WinsockRuntime {
    pub fn init() -> Result<(), TransportError> {
        if STARTED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        #[cfg(windows)]
        {
            use super::ffi::{WSAStartup, WSADATA};
            unsafe {
                let mut data: WSADATA = std::mem::zeroed();
                let rc = WSAStartup(0x0202, &mut data);
                if rc != 0 {
                    STARTED.store(false, Ordering::SeqCst);
                    return Err(TransportError::Winsock {
                        code: rc,
                        message: "WSAStartup failed".into(),
                    });
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn cleanup() {
        if !STARTED.swap(false, Ordering::SeqCst) {
            return;
        }
        #[cfg(windows)]
        {
            unsafe {
                let _ = super::ffi::WSACleanup();
            }
        }
    }
}
