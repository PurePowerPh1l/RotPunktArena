//! RAII RFCOMM socket — exclusive owner, Drop closes.
//!
//! Connect policy (anti re-pair):
//! - Never SDP during connect when channel is known
//! - With explicit channel: GUID_NULL + port (no service discovery)
//! - Single attempt preferred; no multi-port hammering

use super::error::TransportError;
use super::target::RfcommTarget;
use super::ByteTransport;
use std::time::Duration;

pub struct RfcommSocket {
    #[cfg(windows)]
    raw: super::ffi::SOCKET,
    #[cfg(not(windows))]
    _private: (),
    connected: bool,
    /// RFCOMM channel used for this connection (for caching).
    pub channel: Option<u32>,
}

impl RfcommSocket {
    pub fn connect(target: &RfcommTarget, timeout: Duration) -> Result<Self, TransportError> {
        #[cfg(windows)]
        {
            socket_windows::connect(target, timeout)
        }
        #[cfg(not(windows))]
        {
            let _ = (target, timeout);
            Err(TransportError::NotImplemented(
                "RFCOMM nur unter Windows".into(),
            ))
        }
    }

    pub fn shutdown_socket(&mut self) -> Result<(), TransportError> {
        if !self.connected {
            return Ok(());
        }
        #[cfg(windows)]
        {
            socket_windows::do_shutdown(self.raw)?;
        }
        self.connected = false;
        Ok(())
    }
}

impl ByteTransport for RfcommSocket {
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        if !self.connected {
            return Err(TransportError::Closed);
        }
        #[cfg(windows)]
        {
            socket_windows::read(self.raw, buf, timeout)
        }
        #[cfg(not(windows))]
        {
            let _ = (buf, timeout);
            Err(TransportError::NotImplemented("RFCOMM".into()))
        }
    }

    fn write_all(&mut self, data: &[u8], timeout: Duration) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::Closed);
        }
        #[cfg(windows)]
        {
            socket_windows::write_all(self.raw, data, timeout)
        }
        #[cfg(not(windows))]
        {
            let _ = (data, timeout);
            Err(TransportError::NotImplemented("RFCOMM".into()))
        }
    }

    fn shutdown(&mut self) -> Result<(), TransportError> {
        self.shutdown_socket()
    }
}

impl Drop for RfcommSocket {
    fn drop(&mut self) {
        let _ = self.shutdown_socket();
        #[cfg(windows)]
        {
            unsafe {
                let _ = super::ffi::closesocket(self.raw);
            }
        }
    }
}

#[cfg(windows)]
mod socket_windows {
    use super::*;
    use crate::transport::rfcomm::ffi::*;
    use std::mem::{size_of, zeroed};
    use std::ptr;

    pub fn connect(
        target: &RfcommTarget,
        timeout: Duration,
    ) -> Result<RfcommSocket, TransportError> {
        // Caller chooses timeout (12s warm / 45s after long streak).
        let port = 1u32;
        let mut sock = connect_with_port(target, port, timeout)?;
        sock.channel = Some(port);
        Ok(sock)
    }

    fn connect_with_port(
        target: &RfcommTarget,
        port: u32,
        timeout: Duration,
    ) -> Result<RfcommSocket, TransportError> {
        // Plain RFCOMM connect on known channel — no SO_BTH_AUTHENTICATE, no SDP,
        // no auth callback. Windows uses the existing bond; forcing auth pops PIN UI.
        unsafe {
            let s = socket(AF_BTH as i32, SOCK_STREAM, BTHPROTO_RFCOMM);
            if s == INVALID_SOCKET {
                return Err(map_wsa("socket"));
            }

            let mut nb: u32 = 1;
            if ioctlsocket(s, FIONBIO_CMD, &mut nb) == SOCKET_ERROR {
                let e = map_wsa("ioctlsocket");
                abortive_close(s);
                return Err(e);
            }

            let mut addr: SOCKADDR_BTH = zeroed();
            addr.addressFamily = AF_BTH;
            addr.btAddr = target.bt_addr;
            // Explicit RFCOMM channel: do NOT set service GUID (avoids SDP / re-pair prompts).
            addr.serviceClassId = zeroed();
            addr.port = port;

            let rc = crate::transport::rfcomm::ffi::connect(
                s,
                &addr as *const SOCKADDR_BTH as *const SOCKADDR,
                size_of::<SOCKADDR_BTH>() as i32,
            );
            if rc == SOCKET_ERROR {
                let err = WSAGetLastError();
                if err != WSAEWOULDBLOCK {
                    abortive_close(s);
                    return Err(map_connect_err(TransportError::Winsock {
                        code: err,
                        message: format!("connect port={port}"),
                    }));
                }
            }

            if let Err(e) = wait_connected(s, timeout) {
                abortive_close(s);
                return Err(e);
            }

            let mut nb0: u32 = 0;
            let _ = ioctlsocket(s, FIONBIO_CMD, &mut nb0);
            set_timeo(s, SO_RCVTIMEO, timeout)?;
            set_timeo(s, SO_SNDTIMEO, timeout)?;

            Ok(RfcommSocket {
                raw: s,
                connected: true,
                channel: Some(port),
            })
        }
    }

    /// Abort pending RFCOMM page/auth: linger 0 + closesocket.
    /// Plain closesocket often leaves the channel busy (ADDRINUSE / AuthEx spam).
    unsafe fn abortive_close(s: SOCKET) {
        let linger = LINGER {
            l_onoff: 1,
            l_linger: 0,
        };
        let _ = setsockopt(
            s,
            SOL_SOCKET,
            SO_LINGER,
            &linger as *const LINGER as *const u8,
            size_of::<LINGER>() as i32,
        );
        let _ = closesocket(s);
    }

    unsafe fn wait_connected(s: SOCKET, timeout: Duration) -> Result<(), TransportError> {
        let mut write_set: FD_SET = zeroed();
        write_set.fd_count = 1;
        write_set.fd_array[0] = s;
        let mut err_set = write_set;
        let tv = TIMEVAL {
            tv_sec: timeout.as_secs() as i32,
            tv_usec: timeout.subsec_micros() as i32,
        };
        let rc = select(0, ptr::null_mut(), &mut write_set, &mut err_set, &tv);
        if rc == 0 {
            return Err(TransportError::Timeout);
        }
        if rc == SOCKET_ERROR {
            return Err(map_wsa("select"));
        }
        let mut so_err: i32 = 0;
        let mut len = size_of::<i32>() as i32;
        let rc = getsockopt(
            s,
            SOL_SOCKET,
            SO_ERROR,
            &mut so_err as *mut i32 as *mut u8,
            &mut len,
        );
        if rc == SOCKET_ERROR {
            return Err(map_wsa("getsockopt SO_ERROR"));
        }
        if so_err != 0 {
            return Err(map_connect_err(TransportError::Winsock {
                code: so_err,
                message: "SO_ERROR after connect".into(),
            }));
        }
        Ok(())
    }

    unsafe fn set_timeo(s: SOCKET, opt: i32, t: Duration) -> Result<(), TransportError> {
        let ms: u32 = t.as_millis().min(u32::MAX as u128) as u32;
        let bytes = ms.to_le_bytes();
        if setsockopt(s, SOL_SOCKET, opt, bytes.as_ptr(), bytes.len() as i32) == SOCKET_ERROR {
            return Err(map_wsa("setsockopt timeout"));
        }
        Ok(())
    }

    pub fn read(s: SOCKET, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        unsafe {
            let _ = set_timeo(s, SO_RCVTIMEO, timeout);
            let n = recv(s, buf.as_mut_ptr(), buf.len() as i32, 0);
            if n == SOCKET_ERROR {
                let e = WSAGetLastError();
                if e == WSAETIMEDOUT {
                    return Err(TransportError::Timeout);
                }
                return Err(map_connect_err(TransportError::Winsock {
                    code: e,
                    message: "recv".into(),
                }));
            }
            if n == 0 {
                return Err(TransportError::RemoteDisconnected);
            }
            Ok(n as usize)
        }
    }

    pub fn write_all(s: SOCKET, data: &[u8], timeout: Duration) -> Result<(), TransportError> {
        unsafe {
            let _ = set_timeo(s, SO_SNDTIMEO, timeout);
            let mut off = 0;
            while off < data.len() {
                let n = send(s, data[off..].as_ptr(), (data.len() - off) as i32, 0);
                if n == SOCKET_ERROR {
                    return Err(map_connect_err(TransportError::Winsock {
                        code: WSAGetLastError(),
                        message: "send".into(),
                    }));
                }
                off += n as usize;
            }
            Ok(())
        }
    }

    pub fn do_shutdown(s: SOCKET) -> Result<(), TransportError> {
        unsafe {
            if crate::transport::rfcomm::ffi::shutdown(s, SD_BOTH) == SOCKET_ERROR {
                let e = WSAGetLastError();
                if e != WSAENOTCONN {
                    return Err(TransportError::Winsock {
                        code: e,
                        message: "shutdown".into(),
                    });
                }
            }
        }
        Ok(())
    }

    fn map_wsa(op: &str) -> TransportError {
        unsafe {
            TransportError::Winsock {
                code: WSAGetLastError(),
                message: op.into(),
            }
        }
    }

    fn map_connect_err(e: TransportError) -> TransportError {
        // Keep the raw Winsock code — diagnostics need it. Enrich the message.
        match e {
            TransportError::Winsock { code, message } => {
                let label = match code {
                    WSAETIMEDOUT => "PAGE_TIMEOUT / Ziel nicht erreichbar",
                    WSAEDISCON => "RFCOMM remote disconnect",
                    WSAECONNRESET => "RFCOMM session reset",
                    WSAECONNABORTED => "local shutdown",
                    WSAEHOSTDOWN => "RFCOMM DM / host down",
                    WSAEACCES => "auth failed (WSAEACCES)",
                    10048 => "WSAEADDRINUSE / Kanal belegt",
                    10061 => "WSAECONNREFUSED / Service abgelehnt",
                    WSAEINVAL => "stack/device changed (WSAEINVAL)",
                    _ => "winsock",
                };
                TransportError::Winsock {
                    code,
                    message: format!("{label} ({message})"),
                }
            }
            other => other,
        }
    }
}
