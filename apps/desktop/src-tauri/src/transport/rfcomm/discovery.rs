//! Bluetooth discovery for RedDot Classic SPP targets.
//!
//! **Product path needs:** name-hint match, `find_reddot_candidate` /
//! `find_nearby_reddot`, `bond_state`, `pair_with_pin`, `remove_bond`,
//! `resolve_spp` (channel forced to 1 by caller). Persist BD_ADDR from the
//! chosen device — never a hardcoded address (other S/N = other BD_ADDR).
//!
//! Extra enumerate detail is for lab (`bt_diag`); Connect never runs SDP.

use super::error::TransportError;
use super::target::{RfcommTarget, SPP_SERVICE_UUID};

const NAME_HINTS: &[&str] = &["KT RDT", "RDT", "REDDOT", "DISAG"];
/// Classic SPP PIN used by KT RDT / Disag RedDot targets.
pub const REDDOT_PAIR_PIN: &str = "0000";

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub bt_addr: u64,
    pub display_name: String,
    pub paired: bool,
}

/// Lower = better. `None` if name does not look like a RedDot target.
///
/// Hints match at token boundaries (token == hint or token starts with hint,
/// e.g. "RDT203"), never as a raw substring — otherwise unrelated devices
/// whose names merely contain "RDT" inside a word (e.g. "SMARDTV") would be
/// treated as RedDot targets and Nuclear Forget could remove their bond.
pub fn name_hint_rank(display_name: &str) -> Option<usize> {
    let upper = display_name.to_uppercase();
    let tokens: Vec<&str> = upper
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    NAME_HINTS.iter().position(|hint| {
        let hint_tokens: Vec<&str> = hint.split_whitespace().collect();
        if hint_tokens.is_empty() || tokens.len() < hint_tokens.len() {
            return false;
        }
        tokens.windows(hint_tokens.len()).any(|window| {
            let (last, head) = hint_tokens.split_last().expect("non-empty hint");
            window[..head.len()] == *head && window[head.len()].starts_with(last)
        })
    })
}

#[derive(Debug, Clone)]
pub struct DiscoveredDeviceExt {
    pub display_name: String,
    pub bt_addr: u64,
    pub paired: bool,
    pub authenticated: bool,
    pub remembered: bool,
    pub connected: bool,
}

#[derive(Debug, Default)]
pub struct DiscoveryReport {
    pub wsalookup: Vec<DiscoveredDevice>,
    pub bt_find: Vec<DiscoveredDeviceExt>,
    pub pnp: Vec<DiscoveredDevice>,
    pub merged: Vec<DiscoveredDevice>,
}

pub fn enumerate_paired() -> Result<Vec<DiscoveredDevice>, TransportError> {
    Ok(enumerate_paired_detailed()?.merged)
}

pub fn enumerate_paired_detailed() -> Result<DiscoveryReport, TransportError> {
    #[cfg(windows)]
    {
        let _ = crate::transport::rfcomm::WinsockRuntime::init();
        discovery_windows::enumerate_detailed()
    }
    #[cfg(not(windows))]
    {
        Err(TransportError::NotImplemented(
            "Discovery nur unter Windows".into(),
        ))
    }
}

pub fn resolve_spp(bt_addr: u64, display_name: &str) -> Result<RfcommTarget, TransportError> {
    Ok(RfcommTarget {
        bt_addr,
        display_name: display_name.to_string(),
        service_uuid: SPP_SERVICE_UUID.to_string(),
        rfcomm_channel: None,
        com_port: None,
    })
}

pub fn find_reddot_candidate() -> Result<Option<RfcommTarget>, TransportError> {
    let devices = enumerate_paired()?;
    let mut ranked: Vec<_> = devices
        .into_iter()
        .filter(|d| d.bt_addr != 0)
        .filter(|d| name_hint_rank(&d.display_name).is_some())
        .collect();
    ranked.sort_by_key(|d| {
        (
            name_hint_rank(&d.display_name).unwrap_or(99),
            d.display_name.clone(),
        )
    });
    let Some(best) = ranked.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(resolve_spp(best.bt_addr, &best.display_name)?))
}

/// Active inquiry for nearby (possibly unpaired) RedDot-like devices.
pub fn find_nearby_reddot() -> Result<Option<DiscoveredDevice>, TransportError> {
    #[cfg(windows)]
    {
        let _ = crate::transport::rfcomm::WinsockRuntime::init();
        discovery_windows::inquire_reddot_candidate()
    }
    #[cfg(not(windows))]
    {
        Err(TransportError::NotImplemented(
            "Nearby-Discovery nur unter Windows".into(),
        ))
    }
}

/// Pair classic BT with fixed PIN (transparent — no Windows wizard when PIN set).
pub fn pair_with_pin(bt_addr: u64, display_name: &str, pin: &str) -> Result<(), TransportError> {
    #[cfg(windows)]
    {
        let _ = crate::transport::rfcomm::WinsockRuntime::init();
        discovery_windows::authenticate_pin(bt_addr, display_name, pin).map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let _ = (bt_addr, display_name, pin);
        Err(TransportError::NotImplemented(
            "Pairing nur unter Windows".into(),
        ))
    }
}

/// Diagnose-only Pair outcome for labs (e.g. `bt_start_pair_variants`).
///
/// Not used on the product Owner/Nuclear path — prefer [`pair_with_pin`] there.
/// Answers: fresh Success vs already-bonded no-op vs Error (Win32 codes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairApiReport {
    /// Win32 authenticate succeeded (fresh pair).
    Success { win32: u32 },
    /// Bond already authenticated — API skipped or returned already-bonded code.
    AlreadyAuthenticated { reason: &'static str, win32: Option<u32> },
    Error { win32: Option<u32>, message: String },
}

/// Same Win32 path as [`pair_with_pin`], with no-op classification for lab JSONL.
pub fn pair_with_pin_report(
    bt_addr: u64,
    display_name: &str,
    pin: &str,
) -> PairApiReport {
    #[cfg(windows)]
    {
        let _ = crate::transport::rfcomm::WinsockRuntime::init();
        discovery_windows::authenticate_pin_report(bt_addr, display_name, pin)
    }
    #[cfg(not(windows))]
    {
        let _ = (bt_addr, display_name, pin);
        PairApiReport::Error {
            win32: None,
            message: "Pairing nur unter Windows".into(),
        }
    }
}

/// Bond flags from BluetoothFindFirstDevice for a BD_ADDR (if known to the stack).
#[derive(Debug, Clone, Copy, Default)]
pub struct BondState {
    pub remembered: bool,
    pub authenticated: bool,
    pub connected: bool,
}

pub fn bond_state(bt_addr: u64) -> Result<Option<BondState>, TransportError> {
    #[cfg(windows)]
    {
        let _ = crate::transport::rfcomm::WinsockRuntime::init();
        Ok(discovery_windows::bond_state(bt_addr))
    }
    #[cfg(not(windows))]
    {
        let _ = bt_addr;
        Err(TransportError::NotImplemented(
            "bond_state nur unter Windows".into(),
        ))
    }
}

/// Remove the Windows Bluetooth bond for this BD_ADDR (best-effort).
pub fn remove_bond(bt_addr: u64) -> Result<(), TransportError> {
    #[cfg(windows)]
    {
        let _ = crate::transport::rfcomm::WinsockRuntime::init();
        discovery_windows::remove_bond(bt_addr)
    }
    #[cfg(not(windows))]
    {
        let _ = bt_addr;
        Err(TransportError::NotImplemented(
            "remove_bond nur unter Windows".into(),
        ))
    }
}

pub fn parse_addr_from_name(name: &str) -> Option<u64> {
    let bytes = name.as_bytes();
    let mut i = 0;
    while i + 17 <= bytes.len() {
        if looks_like_addr(&bytes[i..i + 17]) {
            return parse_colon_addr(&name[i..i + 17]);
        }
        i += 1;
    }
    None
}

/// Parse `DEV_0018DA070564` style instance fragment → BD_ADDR.
pub fn parse_addr_from_dev_id(id: &str) -> Option<u64> {
    let upper = id.to_uppercase();
    let idx = upper.find("DEV_")?;
    let rest = upper.get(idx + 4..)?;
    let hex: String = rest
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    if hex.len() != 12 {
        return None;
    }
    u64::from_str_radix(&hex, 16).ok()
}

fn looks_like_addr(s: &[u8]) -> bool {
    if s.len() < 17 {
        return false;
    }
    for (i, b) in s.iter().take(17).enumerate() {
        if i % 3 == 2 {
            if *b != b':' {
                return false;
            }
        } else if !b.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn parse_colon_addr(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut addr = 0u64;
    for p in parts {
        let b = u8::from_str_radix(p, 16).ok()?;
        addr = (addr << 8) | u64::from(b);
    }
    Some(addr)
}

fn merge_devices(sets: &[Vec<DiscoveredDevice>]) -> Vec<DiscoveredDevice> {
    let mut by_addr: std::collections::BTreeMap<u64, DiscoveredDevice> =
        std::collections::BTreeMap::new();
    for set in sets {
        for d in set {
            if d.bt_addr == 0 {
                continue;
            }
            by_addr
                .entry(d.bt_addr)
                .and_modify(|e| {
                    if e.display_name.is_empty() && !d.display_name.is_empty() {
                        e.display_name = d.display_name.clone();
                    }
                    e.paired |= d.paired;
                })
                .or_insert_with(|| d.clone());
        }
    }
    by_addr.into_values().collect()
}

#[cfg(windows)]
mod discovery_windows {
    use super::*;
    use crate::transport::rfcomm::ffi::*;
    use std::mem::{size_of, zeroed};
    use std::ptr;

    pub fn enumerate_detailed() -> Result<DiscoveryReport, TransportError> {
        let wsalookup = enumerate_via_wslookup().unwrap_or_default();
        let bt_find_ext = enumerate_via_bluetooth_find().unwrap_or_default();
        let bt_find_simple: Vec<DiscoveredDevice> = bt_find_ext
            .iter()
            .map(|d| DiscoveredDevice {
                bt_addr: d.bt_addr,
                display_name: d.display_name.clone(),
                paired: d.paired,
            })
            .collect();
        let pnp = enumerate_via_pnp().unwrap_or_default();
        let merged = merge_devices(&[wsalookup.clone(), bt_find_simple, pnp.clone()]);
        Ok(DiscoveryReport {
            wsalookup,
            bt_find: bt_find_ext,
            pnp,
            merged,
        })
    }

    fn enumerate_via_wslookup() -> Result<Vec<DiscoveredDevice>, TransportError> {
        unsafe {
            let mut qs: WSAQUERYSETW = zeroed();
            qs.dwSize = size_of::<WSAQUERYSETW>() as u32;
            qs.dwNameSpace = NS_BTH;

            let mut handle: HANDLE = ptr::null_mut();
            let rc = WSALookupServiceBeginW(&mut qs, LUP_CONTAINERS, &mut handle);
            if rc != 0 {
                return Err(TransportError::Winsock {
                    code: WSAGetLastError(),
                    message: "WSALookupServiceBeginW".into(),
                });
            }

            let next_flags = LUP_CONTAINERS | LUP_RETURN_NAME | LUP_RETURN_ADDR;
            let mut out = Vec::new();
            let mut buf = vec![0u8; 16_384];
            loop {
                let mut len = buf.len() as u32;
                let qs_ptr = buf.as_mut_ptr() as *mut WSAQUERYSETW;
                std::ptr::write_bytes(qs_ptr as *mut u8, 0, buf.len());
                (*qs_ptr).dwSize = size_of::<WSAQUERYSETW>() as u32;
                let rc = WSALookupServiceNextW(handle, next_flags, &mut len, qs_ptr);
                if rc != 0 {
                    let err = WSAGetLastError();
                    if err == WSA_E_NO_MORE || err == WSAENOMORE || err == 10108 {
                        break;
                    }
                    if (err == WSAEFAULT || err == 10014) && (len as usize) > buf.len() {
                        buf.resize((len as usize).max(buf.len() * 2), 0);
                        continue;
                    }
                    let _ = WSALookupServiceEnd(handle);
                    return Err(TransportError::Winsock {
                        code: err,
                        message: "WSALookupServiceNextW".into(),
                    });
                }
                if let Some(dev) = parse_queryset(qs_ptr) {
                    out.push(dev);
                }
            }
            let _ = WSALookupServiceEnd(handle);
            Ok(out)
        }
    }

    unsafe fn parse_queryset(qs: *const WSAQUERYSETW) -> Option<DiscoveredDevice> {
        let qs = std::ptr::read_unaligned(qs);
        let name = if !qs.lpszServiceInstanceName.is_null() {
            wide_to_string(qs.lpszServiceInstanceName)
        } else {
            String::new()
        };

        let mut bt_addr = 0u64;
        if !qs.lpcsaBuffer.is_null() && qs.dwNumberOfCsAddrs > 0 {
            let csa = std::ptr::read_unaligned(qs.lpcsaBuffer);
            if !csa.RemoteAddr.lpSockaddr.is_null() {
                bt_addr = read_bth_addr(csa.RemoteAddr.lpSockaddr as *const u8);
            }
        }
        if bt_addr == 0 {
            bt_addr = parse_addr_from_name(&name).unwrap_or(0);
        }
        let display_name = strip_addr_suffix(&name);
        if bt_addr == 0 {
            return None;
        }
        Some(DiscoveredDevice {
            bt_addr,
            display_name,
            paired: true,
        })
    }

    unsafe fn read_bth_addr(p: *const u8) -> u64 {
        if p.is_null() {
            return 0;
        }
        let family = u16::from_le_bytes([*p, *p.add(1)]);
        if family != AF_BTH {
            return 0;
        }
        let aligned = std::ptr::read_unaligned(p as *const SOCKADDR_BTH);
        if aligned.btAddr != 0 {
            return aligned.btAddr & 0xFFFF_FFFF_FFFF;
        }
        let mut raw = [0u8; 8];
        std::ptr::copy_nonoverlapping(p.add(2), raw.as_mut_ptr(), 6);
        u64::from_le_bytes(raw) & 0xFFFF_FFFF_FFFF
    }

    fn strip_addr_suffix(name: &str) -> String {
        let trimmed = name.trim();
        if let Some(open) = trimmed.rfind(" (") {
            let rest = &trimmed[open + 2..];
            if rest.len() >= 18
                && rest.as_bytes().get(17) == Some(&b')')
                && looks_like_addr(rest.as_bytes())
            {
                return trimmed[..open].to_string();
            }
        }
        trimmed.to_string()
    }

    unsafe fn wide_to_string(p: *mut u16) -> String {
        let mut len = 0usize;
        while std::ptr::read_unaligned(p.add(len)) != 0 {
            len += 1;
            if len > 512 {
                break;
            }
        }
        let mut words = Vec::with_capacity(len);
        for i in 0..len {
            words.push(std::ptr::read_unaligned(p.add(i)));
        }
        String::from_utf16_lossy(&words)
    }

    // --- BluetoothFindFirstDevice ---

    #[repr(C)]
    struct BLUETOOTH_ADDRESS {
        ull_long: u64,
    }

    #[repr(C)]
    struct SYSTEMTIME {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }

    #[repr(C)]
    struct BLUETOOTH_DEVICE_INFO {
        dw_size: u32,
        address: BLUETOOTH_ADDRESS,
        ul_classof_device: u32,
        f_connected: i32,
        f_remembered: i32,
        f_authenticated: i32,
        st_last_seen: SYSTEMTIME,
        st_last_used: SYSTEMTIME,
        sz_name: [u16; 248],
    }

    #[repr(C)]
    struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
        dw_size: u32,
        f_return_authenticated: i32,
        f_return_remembered: i32,
        f_return_unknown: i32,
        f_return_connected: i32,
        f_issue_inquiry: i32,
        c_timeout_multiplier: u8,
        _pad: [u8; 7],
        h_radio: *mut std::ffi::c_void,
    }

    #[link(name = "bthprops")]
    extern "system" {
        fn BluetoothFindFirstDevice(
            search: *const BLUETOOTH_DEVICE_SEARCH_PARAMS,
            info: *mut BLUETOOTH_DEVICE_INFO,
        ) -> *mut std::ffi::c_void;
        fn BluetoothFindNextDevice(
            find: *mut std::ffi::c_void,
            info: *mut BLUETOOTH_DEVICE_INFO,
        ) -> i32;
        fn BluetoothFindDeviceClose(find: *mut std::ffi::c_void) -> i32;
        fn BluetoothAuthenticateDevice(
            hwnd_parent: *mut std::ffi::c_void,
            h_radio: *mut std::ffi::c_void,
            pbtdi: *mut BLUETOOTH_DEVICE_INFO,
            psz_passkey: *const u16,
            ul_passkey_length: u32,
        ) -> u32;
        fn BluetoothGetDeviceInfo(
            h_radio: *mut std::ffi::c_void,
            pbtdi: *mut BLUETOOTH_DEVICE_INFO,
        ) -> u32;
        fn BluetoothRemoveDevice(paddress: *const BLUETOOTH_ADDRESS) -> u32;
    }

    fn utf16_z(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn device_name(info: &BLUETOOTH_DEVICE_INFO) -> String {
        let mut len = 0usize;
        while len < info.sz_name.len() && info.sz_name[len] != 0 {
            len += 1;
        }
        String::from_utf16_lossy(&info.sz_name[..len])
    }

    fn enumerate_via_bluetooth_find() -> Result<Vec<DiscoveredDeviceExt>, TransportError> {
        unsafe {
            let search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
                dw_size: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
                f_return_authenticated: 1,
                f_return_remembered: 1,
                f_return_unknown: 0,
                f_return_connected: 1,
                f_issue_inquiry: 0,
                c_timeout_multiplier: 0,
                _pad: [0; 7],
                h_radio: ptr::null_mut(),
            };
            collect_find_devices(&search)
        }
    }

    unsafe fn collect_find_devices(
        search: &BLUETOOTH_DEVICE_SEARCH_PARAMS,
    ) -> Result<Vec<DiscoveredDeviceExt>, TransportError> {
        let mut info: BLUETOOTH_DEVICE_INFO = zeroed();
        info.dw_size = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;

        let find = BluetoothFindFirstDevice(search, &mut info);
        if find.is_null() {
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        loop {
            let addr = info.address.ull_long & 0xFFFF_FFFF_FFFF;
            if addr != 0 {
                out.push(DiscoveredDeviceExt {
                    bt_addr: addr,
                    display_name: device_name(&info),
                    paired: info.f_authenticated != 0 || info.f_remembered != 0,
                    authenticated: info.f_authenticated != 0,
                    remembered: info.f_remembered != 0,
                    connected: info.f_connected != 0,
                });
            }
            info = zeroed();
            info.dw_size = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
            if BluetoothFindNextDevice(find, &mut info) == 0 {
                break;
            }
        }
        let _ = BluetoothFindDeviceClose(find);
        Ok(out)
    }

    /// Inquiry (~8s): find best nearby RedDot by name hints (paired or not).
    pub fn inquire_reddot_candidate() -> Result<Option<DiscoveredDevice>, TransportError> {
        unsafe {
            let search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
                dw_size: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
                f_return_authenticated: 1,
                f_return_remembered: 1,
                f_return_unknown: 1,
                f_return_connected: 1,
                f_issue_inquiry: 1,
                // Each unit ≈ 1.28s → 6 ≈ 7.7s.
                c_timeout_multiplier: 6,
                _pad: [0; 7],
                h_radio: ptr::null_mut(),
            };
            let mut devices = collect_find_devices(&search)?;
            // Refresh empty names via GetDeviceInfo when possible.
            for d in &mut devices {
                if d.display_name.trim().is_empty() {
                    if let Some(name) = refresh_device_name(d.bt_addr) {
                        d.display_name = name;
                    }
                }
            }
            let mut ranked: Vec<_> = devices
                .into_iter()
                .filter(|d| d.bt_addr != 0)
                .filter(|d| super::name_hint_rank(&d.display_name).is_some())
                .collect();
            ranked.sort_by_key(|d| {
                (
                    super::name_hint_rank(&d.display_name).unwrap_or(99),
                    d.display_name.clone(),
                )
            });
            Ok(ranked.into_iter().next().map(|d| DiscoveredDevice {
                bt_addr: d.bt_addr,
                display_name: d.display_name,
                paired: d.paired,
            }))
        }
    }

    fn refresh_device_name(bt_addr: u64) -> Option<String> {
        unsafe {
            let mut info: BLUETOOTH_DEVICE_INFO = zeroed();
            info.dw_size = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
            info.address.ull_long = bt_addr & 0xFFFF_FFFF_FFFF;
            let rc = BluetoothGetDeviceInfo(ptr::null_mut(), &mut info);
            if rc != 0 {
                return None;
            }
            let name = device_name(&info);
            if name.trim().is_empty() {
                None
            } else {
                Some(name)
            }
        }
    }

    pub fn authenticate_pin(
        bt_addr: u64,
        display_name: &str,
        pin: &str,
    ) -> Result<u32, TransportError> {
        match authenticate_pin_report(bt_addr, display_name, pin) {
            super::PairApiReport::Success { win32 } => Ok(win32),
            super::PairApiReport::AlreadyAuthenticated { .. } => Ok(0),
            super::PairApiReport::Error { message, .. } => Err(TransportError::Io(message)),
        }
    }

    pub fn authenticate_pin_report(
        bt_addr: u64,
        display_name: &str,
        pin: &str,
    ) -> super::PairApiReport {
        unsafe {
            if let Some(b) = bond_state(bt_addr) {
                if b.authenticated {
                    return super::PairApiReport::AlreadyAuthenticated {
                        reason: "precheck_bond_authenticated",
                        win32: None,
                    };
                }
            }

            let mut info: BLUETOOTH_DEVICE_INFO = zeroed();
            info.dw_size = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
            info.address.ull_long = bt_addr & 0xFFFF_FFFF_FFFF;
            let name_u16: Vec<u16> = display_name.encode_utf16().collect();
            let copy_len = name_u16.len().min(info.sz_name.len().saturating_sub(1));
            info.sz_name[..copy_len].copy_from_slice(&name_u16[..copy_len]);
            let _ = BluetoothGetDeviceInfo(ptr::null_mut(), &mut info);

            let pin_u16 = utf16_z(pin);
            let pin_chars = pin_u16.len().saturating_sub(1) as u32;
            // Prefer the auth-hook radio handle — NULL often yields flaky PIN UI on Win11.
            let radio = crate::transport::rfcomm::auth_hook::local_radio_handle()
                .map(|h| h.0 as *mut std::ffi::c_void)
                .unwrap_or(ptr::null_mut());
            let rc = BluetoothAuthenticateDevice(
                ptr::null_mut(),
                radio,
                &mut info,
                pin_u16.as_ptr(),
                pin_chars,
            );
            // 0 = success; 183 = ERROR_ALREADY_EXISTS; 259 = ERROR_NO_MORE_ITEMS (often already auth).
            if rc == 0 {
                return super::PairApiReport::Success { win32: rc };
            }
            if rc == 183 || rc == 259 {
                return super::PairApiReport::AlreadyAuthenticated {
                    reason: "win32_already_bonded",
                    win32: Some(rc),
                };
            }
            if let Some(b) = bond_state(bt_addr) {
                if b.authenticated {
                    return super::PairApiReport::AlreadyAuthenticated {
                        reason: "bond_auth_after_fail",
                        win32: Some(rc),
                    };
                }
            }
            super::PairApiReport::Error {
                win32: Some(rc),
                message: format!(
                    "Bluetooth-Pairing fehlgeschlagen (Win32 {rc}) — Ziel an und sichtbar?"
                ),
            }
        }
    }

    pub fn bond_state(bt_addr: u64) -> Option<BondState> {
        let want = bt_addr & 0xFFFF_FFFF_FFFF;
        let devices = enumerate_via_bluetooth_find().ok()?;
        devices.into_iter().find(|d| d.bt_addr == want).map(|d| BondState {
            remembered: d.remembered || d.paired,
            authenticated: d.authenticated,
            connected: d.connected,
        })
    }

    pub fn remove_bond(bt_addr: u64) -> Result<(), TransportError> {
        unsafe {
            let addr = BLUETOOTH_ADDRESS {
                ull_long: bt_addr & 0xFFFF_FFFF_FFFF,
            };
            let rc = BluetoothRemoveDevice(&addr);
            // 0 = success; 1168 = ERROR_NOT_FOUND (already gone) — treat as Ok.
            if rc == 0 || rc == 1168 {
                return Ok(());
            }
            Err(TransportError::Io(format!(
                "BluetoothRemoveDevice failed ({rc})"
            )))
        }
    }

    // --- PnP: BTHENUM\DEV_<12 hex> ---

    #[link(name = "setupapi")]
    extern "system" {
        fn SetupDiGetClassDevsW(
            class_guid: *const GUID,
            enumerator: *const u16,
            hwnd: *mut std::ffi::c_void,
            flags: u32,
        ) -> *mut std::ffi::c_void;
        fn SetupDiEnumDeviceInfo(
            devs: *mut std::ffi::c_void,
            index: u32,
            info: *mut SP_DEVINFO_DATA,
        ) -> i32;
        fn SetupDiGetDeviceInstanceIdW(
            devs: *mut std::ffi::c_void,
            info: *mut SP_DEVINFO_DATA,
            id: *mut u16,
            id_size: u32,
            required: *mut u32,
        ) -> i32;
        fn SetupDiGetDeviceRegistryPropertyW(
            devs: *mut std::ffi::c_void,
            info: *const SP_DEVINFO_DATA,
            prop: u32,
            reg_type: *mut u32,
            buf: *mut u8,
            buf_size: u32,
            required: *mut u32,
        ) -> i32;
        fn SetupDiDestroyDeviceInfoList(devs: *mut std::ffi::c_void) -> i32;
    }

    #[repr(C)]
    struct SP_DEVINFO_DATA {
        cb_size: u32,
        class_guid: GUID,
        dev_inst: u32,
        reserved: usize,
    }

    const DIGCF_PRESENT: u32 = 0x00000002;
    const DIGCF_ALLCLASSES: u32 = 0x00000004;
    const SPDRP_FRIENDLYNAME: u32 = 0x0000000C;
    const SPDRP_DEVICEDESC: u32 = 0x00000000;

    /// Bluetooth class GUID {e0cbf06c-cd8b-4647-bb8a-263b43f0f974}
    fn bluetooth_class_guid() -> GUID {
        GUID {
            Data1: 0xe0cbf06c,
            Data2: 0xcd8b,
            Data3: 0x4647,
            Data4: [0xbb, 0x8a, 0x26, 0x3b, 0x43, 0xf0, 0xf9, 0x74],
        }
    }

    fn enumerate_via_pnp() -> Result<Vec<DiscoveredDevice>, TransportError> {
        unsafe {
            let guid = bluetooth_class_guid();
            let mut out = Vec::new();
            // Prefer Bluetooth class; also scan BTHENUM enumerator.
            collect_pnp_devs(
                SetupDiGetClassDevsW(&guid, ptr::null(), ptr::null_mut(), DIGCF_PRESENT),
                &mut out,
            );
            let enum_bthenum: Vec<u16> = "BTHENUM\0".encode_utf16().collect();
            collect_pnp_devs(
                SetupDiGetClassDevsW(
                    ptr::null(),
                    enum_bthenum.as_ptr(),
                    ptr::null_mut(),
                    DIGCF_PRESENT | DIGCF_ALLCLASSES,
                ),
                &mut out,
            );
            Ok(merge_devices(&[out]))
        }
    }

    unsafe fn collect_pnp_devs(devs: *mut std::ffi::c_void, out: &mut Vec<DiscoveredDevice>) {
        if devs.is_null() || devs == (-1isize as *mut std::ffi::c_void) {
            return;
        }
        let mut idx = 0u32;
        loop {
            let mut info: SP_DEVINFO_DATA = zeroed();
            info.cb_size = size_of::<SP_DEVINFO_DATA>() as u32;
            if SetupDiEnumDeviceInfo(devs, idx, &mut info) == 0 {
                break;
            }
            idx += 1;

            let mut id_buf = [0u16; 512];
            let mut required = 0u32;
            if SetupDiGetDeviceInstanceIdW(
                devs,
                &mut info,
                id_buf.as_mut_ptr(),
                id_buf.len() as u32,
                &mut required,
            ) == 0
            {
                continue;
            }
            let id_len = id_buf.iter().position(|&c| c == 0).unwrap_or(id_buf.len());
            let instance_id = String::from_utf16_lossy(&id_buf[..id_len]);
            let Some(bt_addr) = parse_addr_from_dev_id(&instance_id) else {
                continue;
            };

            let friendly = pnp_prop_string(devs, &info, SPDRP_FRIENDLYNAME)
                .or_else(|| pnp_prop_string(devs, &info, SPDRP_DEVICEDESC))
                .unwrap_or_default();

            out.push(DiscoveredDevice {
                bt_addr,
                display_name: friendly,
                paired: true,
            });
        }
        let _ = SetupDiDestroyDeviceInfoList(devs);
    }

    unsafe fn pnp_prop_string(
        devs: *mut std::ffi::c_void,
        info: &SP_DEVINFO_DATA,
        prop: u32,
    ) -> Option<String> {
        let mut reg_type = 0u32;
        let mut required = 0u32;
        let mut buf = vec![0u8; 512];
        if SetupDiGetDeviceRegistryPropertyW(
            devs,
            info,
            prop,
            &mut reg_type,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut required,
        ) == 0
        {
            if required > buf.len() as u32 {
                buf.resize(required as usize, 0);
                if SetupDiGetDeviceRegistryPropertyW(
                    devs,
                    info,
                    prop,
                    &mut reg_type,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    &mut required,
                ) == 0
                {
                    return None;
                }
            } else {
                return None;
            }
        }
        let words = std::slice::from_raw_parts(
            buf.as_ptr() as *const u16,
            (buf.len() / 2).saturating_sub(1),
        );
        let len = words.iter().position(|&c| c == 0).unwrap_or(words.len());
        let s = String::from_utf16_lossy(&words[..len]);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_from_parenthetical_name() {
        let n = "KT RDT ZIE 1 S/N 203 (a1:b2:c3:d4:e5:f6)";
        assert_eq!(parse_addr_from_name(n), Some(0x00a1_b2c3_d4e5_f6));
    }

    #[test]
    fn parse_addr_from_pnp_instance() {
        let id = r"BTHENUM\DEV_0018DA070564\C&B1ED653&0&BLUETOOTHDEVICE_0018DA070564";
        assert_eq!(parse_addr_from_dev_id(id), Some(0x0018_DA07_0564));
    }

    #[test]
    fn name_hint_matches_reddot_names() {
        assert_eq!(name_hint_rank("KT RDT ZIE 1 S/N 203"), Some(0));
        assert!(name_hint_rank("RDT203").is_some());
        assert!(name_hint_rank("DISAG RedDot").is_some());
        assert!(name_hint_rank("reddot-stand").is_some());
    }

    #[test]
    fn name_hint_rejects_substring_lookalikes() {
        assert_eq!(name_hint_rank("SMARDTV Box"), None);
        assert_eq!(name_hint_rank("HardTop Speaker"), None);
        assert_eq!(name_hint_rank(""), None);
    }
}
