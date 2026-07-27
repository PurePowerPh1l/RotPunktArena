//! Raw Winsock + Bluetooth constants (avoid windows-crate version skew with Tauri).

#![allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::upper_case_acronyms
)]

use std::ffi::c_void;

pub type SOCKET = usize;
pub type DWORD = u32;
pub type WORD = u16;
pub type INT = i32;
pub type ULONG = u32;
pub type SHORT = i16;
pub type USHORT = u16;
pub type ADDRESS_FAMILY = u16;
pub type HANDLE = *mut c_void;

pub const INVALID_SOCKET: SOCKET = !0;
pub const SOCKET_ERROR: i32 = -1;
pub const AF_BTH: ADDRESS_FAMILY = 32;
pub const SOCK_STREAM: i32 = 1;
pub const BTHPROTO_RFCOMM: i32 = 3;
pub const SOL_SOCKET: i32 = 0xffff;
pub const SO_ERROR: i32 = 0x1007;
pub const SO_RCVTIMEO: i32 = 0x1006;
pub const SO_SNDTIMEO: i32 = 0x1005;
pub const SO_LINGER: i32 = 0x0080;

/// Winsock `struct linger` for abortive close (`l_onoff=1`, `l_linger=0`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LINGER {
    pub l_onoff: u16,
    pub l_linger: u16,
}
pub const WSAEACCES: i32 = 10013;
pub const SD_BOTH: i32 = 2;
pub const FIONBIO: i32 = -2147195266; // IOC_IN | 0x8004667e pattern used by WinSock
pub const NS_BTH: DWORD = 16;
pub const LUP_CONTAINERS: DWORD = 0x0002;
pub const LUP_RETURN_NAME: DWORD = 0x0010;
pub const LUP_RETURN_ADDR: DWORD = 0x0100;
pub const LUP_FLUSHCACHE: DWORD = 0x1000;
pub const WSAEWOULDBLOCK: i32 = 10035;
pub const WSAETIMEDOUT: i32 = 10060;
pub const WSAECONNRESET: i32 = 10054;
pub const WSAECONNABORTED: i32 = 10053;
pub const WSAEHOSTDOWN: i32 = 10064;
pub const WSAEINVAL: i32 = 10022;
pub const WSAEDISCON: i32 = 10101;
pub const WSAENOMORE: i32 = 10102;
pub const WSA_E_NO_MORE: i32 = 10110;
pub const WSAEFAULT: i32 = 10014;
pub const WSAENOTCONN: i32 = 10057;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WSADATA {
    pub wVersion: WORD,
    pub wHighVersion: WORD,
    pub iMaxSockets: u16,
    pub iMaxUdpDg: u16,
    pub lpVendorInfo: *mut i8,
    pub szDescription: [i8; 257],
    pub szSystemStatus: [i8; 129],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

/// Bluetooth socket address — **packed** (ws2bth.h / Windows Bluetooth samples).
/// Must NOT insert padding after `addressFamily` or connect() sees a garbage BD_ADDR.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SOCKADDR_BTH {
    pub addressFamily: ADDRESS_FAMILY,
    pub btAddr: u64,
    pub serviceClassId: GUID,
    pub port: ULONG,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SOCKADDR {
    pub sa_family: ADDRESS_FAMILY,
    pub sa_data: [i8; 14],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TIMEVAL {
    pub tv_sec: i32,
    pub tv_usec: i32,
}

pub const FD_SETSIZE: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FD_SET {
    pub fd_count: u32,
    pub fd_array: [SOCKET; FD_SETSIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SOCKET_ADDRESS {
    pub lpSockaddr: *mut SOCKADDR,
    pub iSockaddrLength: i32,
    /// x64 padding to 16 bytes (matches WinSock SOCKET_ADDRESS).
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CSADDR_INFO {
    pub LocalAddr: SOCKET_ADDRESS,
    pub RemoteAddr: SOCKET_ADDRESS,
    pub iSocketType: i32,
    pub iProtocol: i32,
}

#[repr(C)]
pub struct WSAQUERYSETW {
    pub dwSize: DWORD,
    pub lpszServiceInstanceName: *mut u16,
    pub lpServiceClassId: *mut GUID,
    pub lpVersion: *mut c_void,
    pub lpszComment: *mut u16,
    pub dwNameSpace: DWORD,
    pub lpNSProviderId: *mut GUID,
    pub lpszContext: *mut u16,
    pub dwNumberOfProtocols: DWORD,
    pub lpafpProtocols: *mut c_void,
    pub lpszQueryString: *mut u16,
    pub dwNumberOfCsAddrs: DWORD,
    pub lpcsaBuffer: *mut CSADDR_INFO,
    pub dwOutputFlags: DWORD,
    pub lpBlob: *mut c_void,
}

#[link(name = "ws2_32")]
extern "system" {
    pub fn WSAStartup(wVersionRequested: WORD, lpWSAData: *mut WSADATA) -> i32;
    pub fn WSACleanup() -> i32;
    pub fn WSAGetLastError() -> i32;
    pub fn socket(af: i32, socket_type: i32, protocol: i32) -> SOCKET;
    pub fn closesocket(s: SOCKET) -> i32;
    pub fn connect(s: SOCKET, name: *const SOCKADDR, namelen: i32) -> i32;
    pub fn shutdown(s: SOCKET, how: i32) -> i32;
    pub fn recv(s: SOCKET, buf: *mut u8, len: i32, flags: i32) -> i32;
    pub fn send(s: SOCKET, buf: *const u8, len: i32, flags: i32) -> i32;
    pub fn ioctlsocket(s: SOCKET, cmd: i32, argp: *mut u32) -> i32;
    pub fn setsockopt(s: SOCKET, level: i32, optname: i32, optval: *const u8, optlen: i32) -> i32;
    pub fn getsockopt(
        s: SOCKET,
        level: i32,
        optname: i32,
        optval: *mut u8,
        optlen: *mut i32,
    ) -> i32;
    pub fn select(
        nfds: i32,
        readfds: *mut FD_SET,
        writefds: *mut FD_SET,
        exceptfds: *mut FD_SET,
        timeout: *const TIMEVAL,
    ) -> i32;
    pub fn WSALookupServiceBeginW(
        qs: *mut WSAQUERYSETW,
        dwFlags: DWORD,
        lphLookup: *mut HANDLE,
    ) -> i32;
    pub fn WSALookupServiceNextW(
        hLookup: HANDLE,
        dwFlags: DWORD,
        lpdwBufferLength: *mut DWORD,
        qs: *mut WSAQUERYSETW,
    ) -> i32;
    pub fn WSALookupServiceEnd(hLookup: HANDLE) -> i32;
}

pub fn spp_guid() -> GUID {
    GUID {
        Data1: 0x0000_1101,
        Data2: 0x0000,
        Data3: 0x1000,
        Data4: [0x80, 0x00, 0x00, 0x80, 0x5F, 0x9B, 0x34, 0xFB],
    }
}

/// FIONBIO value from WinSock2.h: `_IOW('f', 126, u_long)` = 0x8004667e as i32
pub const FIONBIO_CMD: i32 = 0x8004667Eu32 as i32;
