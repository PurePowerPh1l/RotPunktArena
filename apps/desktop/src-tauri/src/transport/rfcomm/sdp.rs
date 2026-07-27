//! SDP channel resolve — never flush cache (flush triggers Windows re-pair UI).
//!
//! **Not** on the product connect path (kanal is hard-coded to 1).
//! Use for one-off diagnosis if a future hardware revision does not answer on
//! RFCOMM channel 1 — see `docs/bluetooth-connection-stack.md` §10.4.

use super::error::TransportError;
use super::ffi::spp_guid;

/// Resolve RFCOMM channel for SPP. Does **not** flush SDP cache.
pub fn resolve_spp_channel(bt_addr: u64) -> Result<Option<u32>, TransportError> {
    #[cfg(windows)]
    {
        sdp_windows::resolve_spp_channel(bt_addr)
    }
    #[cfg(not(windows))]
    {
        let _ = bt_addr;
        Ok(None)
    }
}

#[cfg(windows)]
mod sdp_windows {
    use super::*;
    use crate::transport::rfcomm::ffi::*;
    use std::mem::{size_of, zeroed};
    use std::ptr;

    pub fn resolve_spp_channel(bt_addr: u64) -> Result<Option<u32>, TransportError> {
        unsafe {
            let ctx = addr_context(bt_addr);
            let mut ctx_wide: Vec<u16> = ctx.encode_utf16().chain(std::iter::once(0)).collect();
            let mut svc = spp_guid();

            let mut qs: WSAQUERYSETW = zeroed();
            qs.dwSize = size_of::<WSAQUERYSETW>() as u32;
            qs.dwNameSpace = NS_BTH;
            qs.lpServiceClassId = &mut svc;
            qs.lpszContext = ctx_wide.as_mut_ptr();

            let mut handle: HANDLE = ptr::null_mut();
            // No LUP_FLUSHCACHE — ever.
            let flags = LUP_RETURN_ADDR;
            if WSALookupServiceBeginW(&mut qs, flags, &mut handle) != 0 {
                return Ok(None);
            }

            let mut buf = vec![0u8; 16_384];
            let mut channel: Option<u32> = None;
            loop {
                let mut len = buf.len() as u32;
                let qs_ptr = buf.as_mut_ptr() as *mut WSAQUERYSETW;
                std::ptr::write_bytes(qs_ptr as *mut u8, 0, buf.len());
                (*qs_ptr).dwSize = size_of::<WSAQUERYSETW>() as u32;
                let rc = WSALookupServiceNextW(handle, flags, &mut len, qs_ptr);
                if rc != 0 {
                    let err = WSAGetLastError();
                    if err == WSA_E_NO_MORE || err == WSAENOMORE || err == 10108 {
                        break;
                    }
                    if (err == WSAEFAULT || err == 10014) && (len as usize) > buf.len() {
                        buf.resize((len as usize).max(buf.len() * 2), 0);
                        continue;
                    }
                    break;
                }
                let q = std::ptr::read_unaligned(qs_ptr);
                if !q.lpcsaBuffer.is_null() && q.dwNumberOfCsAddrs > 0 {
                    let csa = std::ptr::read_unaligned(q.lpcsaBuffer);
                    if !csa.RemoteAddr.lpSockaddr.is_null() {
                        let sa = std::ptr::read_unaligned(
                            csa.RemoteAddr.lpSockaddr as *const SOCKADDR_BTH,
                        );
                        if sa.port != 0 {
                            channel = Some(sa.port);
                            break;
                        }
                    }
                }
            }
            let _ = WSALookupServiceEnd(handle);
            Ok(channel)
        }
    }

    fn addr_context(bt_addr: u64) -> String {
        let b = [
            ((bt_addr >> 40) & 0xff) as u8,
            ((bt_addr >> 32) & 0xff) as u8,
            ((bt_addr >> 24) & 0xff) as u8,
            ((bt_addr >> 16) & 0xff) as u8,
            ((bt_addr >> 8) & 0xff) as u8,
            (bt_addr & 0xff) as u8,
        ];
        format!(
            "({:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X})",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    }
}
