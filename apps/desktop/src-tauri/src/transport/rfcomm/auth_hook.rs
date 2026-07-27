//! Auto-answer classic PIN for RedDot targets when Windows asks.
//!
//! **Product:** install only via [`SetupAuthGuard`] during First-Setup.
//! Everyday RFCOMM reconnect must not register hooks or allow-list addresses.
//!
//! Win11 may deliver classic SPP auth via either
//! `BluetoothRegisterForAuthentication` (legacy) or
//! `BluetoothRegisterForAuthenticationEx`. We register **both** and answer
//! LEGACY with PIN `0000` for allow-listed / name-hint devices.
//!
//! Send order in the Ex callback (fast path — no disk I/O before Send):
//! 1. `SendAuthenticationResponseEx` with the radio opened at install
//! 2. `SendAuthenticationResponseEx(NULL)`
//! 3. Legacy `SendAuthenticationResponse` with deviceInfo + UTF-16 PIN
//!
//! `ERROR_NOT_READY` (21) usually means the request already timed out or the
//! RFCOMM page cancelled it — returning failure then shows the Windows PIN UI
//! (manual `0000` still works).

use super::discovery::{name_hint_rank, REDDOT_PAIR_PIN};
use super::error::TransportError;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

static HOOK: Mutex<Option<AuthHook>> = Mutex::new(None);
static ALLOWED_ADDRS: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
/// Last auto-PIN / auth result (diag for soak / UI).
static LAST_AUTH_NOTE: Mutex<Option<String>> = Mutex::new(None);
/// How many auth callbacks fired since [`reset_auth_callback_count`].
static AUTH_CALLBACK_COUNT: AtomicU32 = AtomicU32::new(0);

fn allowed_addrs() -> &'static Mutex<HashSet<u64>> {
    ALLOWED_ADDRS.get_or_init(|| Mutex::new(HashSet::new()))
}

struct AuthHook {
    #[cfg(windows)]
    reg_ex: isize,
    #[cfg(windows)]
    reg_legacy: isize,
}

unsafe impl Send for AuthHook {}

/// Remember a BD_ADDR that may receive auto-PIN `0000` (known RedDot target).
pub fn allow_auto_pin_for(bt_addr: u64) {
    let addr = bt_addr & 0xFFFF_FFFF_FFFF;
    allowed_addrs().lock().unwrap().insert(addr);
}

pub fn clear_auto_pin_allows() {
    allowed_addrs().lock().unwrap().clear();
}

fn addr_allowed(bt_addr: u64) -> bool {
    let addr = bt_addr & 0xFFFF_FFFF_FFFF;
    allowed_addrs().lock().unwrap().contains(&addr)
}

/// Last auth note (for soak / diagnostics). Cleared on read.
pub fn take_last_auth_note() -> Option<String> {
    LAST_AUTH_NOTE.lock().unwrap().take()
}

/// Reset callback counter and drain the last note before a Nuclear/Pair window.
pub fn reset_auth_callback_count() {
    AUTH_CALLBACK_COUNT.store(0, Ordering::SeqCst);
    let _ = take_last_auth_note();
}

/// Auth callbacks since last reset (swap to 0).
pub fn take_auth_callback_count() -> u32 {
    AUTH_CALLBACK_COUNT.swap(0, Ordering::SeqCst)
}

fn set_auth_note(msg: String) {
    eprintln!("[authHook] {msg}");
    AUTH_CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
    *LAST_AUTH_NOTE.lock().unwrap() = Some(msg.clone());
    // Disk I/O after the in-memory note — never call this *before* Send.
    crate::connection::diag::append_repo(&crate::connection::diag::DiagEvent {
        ts: crate::connection::diag::now_ts(),
        event: "auth_ex",
        status: "connecting",
        reason: &msg,
        generation: None,
        addr: None,
        channel: None,
        winsock: None,
        winsock_name: None,
        attempt: None,
        silent: None,
        auth_hook_installed: None,
        release_attempted: None,
        result: None,
        toast_risk_path_entered: None,
    });
}

/// Register Ex + legacy hooks (idempotent while held by SetupAuthGuard).
pub fn install_reddot_pin_hook() -> Result<(), TransportError> {
    #[cfg(windows)]
    {
        let mut g = HOOK.lock().unwrap();
        if g.is_some() {
            return Ok(());
        }
        let (reg_ex, reg_legacy) = win::register_both()?;
        *g = Some(AuthHook {
            reg_ex,
            reg_legacy,
        });
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

pub fn uninstall_reddot_pin_hook() {
    #[cfg(windows)]
    {
        let mut g = HOOK.lock().unwrap();
        if let Some(hook) = g.take() {
            win::unregister(hook.reg_ex, hook.reg_legacy);
        }
    }
}

/// Handle to the local radio kept open for AuthEx / AuthenticateDevice (if any).
#[cfg(windows)]
pub fn local_radio_handle() -> Option<::windows::Win32::Foundation::HANDLE> {
    win::radio_handle()
}

#[cfg(not(windows))]
pub fn local_radio_handle() -> Option<()> {
    None
}

/// Pair with PIN under a setup-only auth guard (install → pair → Drop uninstall).
///
/// Everyday reconnect must not call this — only First-Setup / lab `bt_pair`.
pub fn pair_with_pin_exclusive(
    bt_addr: u64,
    display_name: &str,
    pin: &str,
) -> Result<(), TransportError> {
    let _guard = SetupAuthGuard::enter(bt_addr)?;
    crate::transport::rfcomm::discovery::pair_with_pin(bt_addr, display_name, pin)
}

/// RAII: Ex+Legacy PIN hook + allow-list only for the Setup pairing window.
///
/// Drop always clears allow-list and uninstalls the hook (Ok / Err / panic unwind).
pub struct SetupAuthGuard {
    _addr: u64,
}

impl SetupAuthGuard {
    pub fn enter(bt_addr: u64) -> Result<Self, TransportError> {
        clear_auto_pin_allows();
        allow_auto_pin_for(bt_addr);
        install_reddot_pin_hook()?;
        Ok(Self {
            _addr: bt_addr & 0xFFFF_FFFF_FFFF,
        })
    }
}

impl Drop for SetupAuthGuard {
    fn drop(&mut self) {
        clear_auto_pin_allows();
        uninstall_reddot_pin_hook();
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use ::windows::core::{BOOL, PCWSTR};
    use ::windows::Win32::Devices::Bluetooth::*;
    use ::windows::Win32::Foundation::{CloseHandle, HANDLE};

    /// Local radio kept open for AuthEx (`0` = none).
    static RADIO_HANDLE: AtomicIsize = AtomicIsize::new(0);
    static RADIO_FIND: AtomicIsize = AtomicIsize::new(0);

    fn device_name(info: &BLUETOOTH_DEVICE_INFO) -> String {
        let mut len = 0usize;
        while len < info.szName.len() && info.szName[len] != 0 {
            len += 1;
        }
        String::from_utf16_lossy(&info.szName[..len])
    }

    fn addr_ull(info: &BLUETOOTH_DEVICE_INFO) -> u64 {
        unsafe { info.Address.Anonymous.ullLong & 0xFFFF_FFFF_FFFF }
    }

    fn should_auto_pin(info: &BLUETOOTH_DEVICE_INFO) -> bool {
        if addr_allowed(addr_ull(info)) {
            return true;
        }
        name_hint_rank(&device_name(info)).is_some()
    }

    fn pin_info() -> BLUETOOTH_PIN_INFO {
        let mut pin = [0u8; 16];
        let raw = REDDOT_PAIR_PIN.as_bytes();
        let n = raw.len().min(16);
        pin[..n].copy_from_slice(&raw[..n]);
        BLUETOOTH_PIN_INFO {
            pin,
            pinLength: n as u8,
        }
    }

    fn pin_utf16() -> Vec<u16> {
        REDDOT_PAIR_PIN
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    }

    fn open_local_radio() -> Result<(), TransportError> {
        close_local_radio();
        unsafe {
            let params = BLUETOOTH_FIND_RADIO_PARAMS {
                dwSize: std::mem::size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
            };
            let mut radio = HANDLE::default();
            let find = BluetoothFindFirstRadio(&params, &mut radio).map_err(|e| {
                TransportError::Io(format!("BluetoothFindFirstRadio failed ({e})"))
            })?;
            if radio.is_invalid() {
                let _ = BluetoothFindRadioClose(find);
                return Err(TransportError::Io(
                    "BluetoothFindFirstRadio returned invalid radio handle".into(),
                ));
            }
            RADIO_HANDLE.store(radio.0 as isize, Ordering::SeqCst);
            RADIO_FIND.store(find.0 as isize, Ordering::SeqCst);
            eprintln!("[authHook] local radio open handle={:?}", radio);
            Ok(())
        }
    }

    fn close_local_radio() {
        let find = RADIO_FIND.swap(0, Ordering::SeqCst);
        let radio = RADIO_HANDLE.swap(0, Ordering::SeqCst);
        unsafe {
            if radio != 0 {
                let _ = CloseHandle(HANDLE(radio as *mut _));
            }
            if find != 0 {
                let _ = BluetoothFindRadioClose(HBLUETOOTH_RADIO_FIND(find as *mut _));
            }
        }
    }

    pub fn radio_handle() -> Option<HANDLE> {
        let v = RADIO_HANDLE.load(Ordering::SeqCst);
        if v == 0 {
            None
        } else {
            Some(HANDLE(v as *mut _))
        }
    }

    /// Answer LEGACY PIN as fast as possible. Radio → NULL Ex → legacy Send.
    fn send_legacy_pin(device: &BLUETOOTH_DEVICE_INFO) -> (u32, &'static str) {
        unsafe {
            let mut resp = BLUETOOTH_AUTHENTICATE_RESPONSE::default();
            resp.bthAddressRemote = device.Address;
            resp.authMethod = BLUETOOTH_AUTHENTICATION_METHOD_LEGACY;
            resp.negativeResponse = 0;
            resp.Anonymous.pinInfo = pin_info();

            if let Some(radio) = radio_handle() {
                let rc = BluetoothSendAuthenticationResponseEx(Some(radio), &resp);
                if rc == 0 {
                    return (rc, "ex_radio");
                }
            }
            let rc_null = BluetoothSendAuthenticationResponseEx(None, &resp);
            if rc_null == 0 {
                return (rc_null, "ex_null");
            }
            let pin = pin_utf16();
            let rc_leg = BluetoothSendAuthenticationResponse(
                radio_handle(),
                device,
                PCWSTR(pin.as_ptr()),
            );
            if rc_leg == 0 {
                return (rc_leg, "legacy");
            }
            // Prefer the most informative non-success code for diag.
            if rc_null != 0 {
                (rc_null, "fail_ex_null")
            } else {
                (rc_leg, "fail_legacy")
            }
        }
    }

    unsafe extern "system" fn on_auth_ex(
        _pv: *const std::ffi::c_void,
        params: *const BLUETOOTH_AUTHENTICATION_CALLBACK_PARAMS,
    ) -> BOOL {
        if params.is_null() {
            return BOOL(0);
        }
        let p = unsafe { &*params };
        if !should_auto_pin(&p.deviceInfo) {
            set_auth_note(format!(
                "authEx skipped method={} name={:?}",
                p.authenticationMethod.0,
                device_name(&p.deviceInfo)
            ));
            return BOOL(0);
        }

        let method = p.authenticationMethod;
        let name = device_name(&p.deviceInfo);
        let addr = addr_ull(&p.deviceInfo);

        let (rc, via) = match method {
            BLUETOOTH_AUTHENTICATION_METHOD_LEGACY => send_legacy_pin(&p.deviceInfo),
            BLUETOOTH_AUTHENTICATION_METHOD_NUMERIC_COMPARISON => unsafe {
                let mut resp = BLUETOOTH_AUTHENTICATE_RESPONSE::default();
                resp.bthAddressRemote = p.deviceInfo.Address;
                resp.authMethod = method;
                resp.negativeResponse = 0;
                resp.Anonymous.numericCompInfo = BLUETOOTH_NUMERIC_COMPARISON_INFO {
                    NumericValue: p.Anonymous.Numeric_Value,
                };
                if let Some(radio) = radio_handle() {
                    let rc = BluetoothSendAuthenticationResponseEx(Some(radio), &resp);
                    if rc == 0 {
                        (rc, "ex_radio")
                    } else {
                        let rc2 = BluetoothSendAuthenticationResponseEx(None, &resp);
                        (rc2, if rc2 == 0 { "ex_null" } else { "fail_numeric" })
                    }
                } else {
                    let rc = BluetoothSendAuthenticationResponseEx(None, &resp);
                    (rc, if rc == 0 { "ex_null" } else { "fail_numeric" })
                }
            },
            BLUETOOTH_AUTHENTICATION_METHOD_PASSKEY_NOTIFICATION
            | BLUETOOTH_AUTHENTICATION_METHOD_PASSKEY => {
                let passkey: u32 = REDDOT_PAIR_PIN.parse().unwrap_or(0);
                let mut resp = BLUETOOTH_AUTHENTICATE_RESPONSE::default();
                resp.bthAddressRemote = p.deviceInfo.Address;
                resp.authMethod = BLUETOOTH_AUTHENTICATION_METHOD_PASSKEY_NOTIFICATION;
                resp.negativeResponse = 0;
                resp.Anonymous.passkeyInfo = BLUETOOTH_PASSKEY_INFO { passkey };
                unsafe {
                    if let Some(radio) = radio_handle() {
                        let rc = BluetoothSendAuthenticationResponseEx(Some(radio), &resp);
                        if rc == 0 {
                            (rc, "ex_radio")
                        } else {
                            let rc2 = BluetoothSendAuthenticationResponseEx(None, &resp);
                            (rc2, if rc2 == 0 { "ex_null" } else { "fail_passkey" })
                        }
                    } else {
                        let rc = BluetoothSendAuthenticationResponseEx(None, &resp);
                        (rc, if rc == 0 { "ex_null" } else { "fail_passkey" })
                    }
                }
            }
            other => {
                set_auth_note(format!("authEx unsupported method={}", other.0));
                return BOOL(0);
            }
        };

        set_auth_note(format!(
            "authEx method={} addr={addr:012X} name={name:?} send_rc={rc} via={via}",
            method.0
        ));
        if rc == 0 {
            BOOL(1)
        } else {
            BOOL(0)
        }
    }

    /// Legacy callback (master path) — Win11 sometimes only fires this.
    unsafe extern "system" fn on_auth_legacy(
        _pv: *mut std::ffi::c_void,
        info: *mut BLUETOOTH_DEVICE_INFO,
    ) -> BOOL {
        if info.is_null() {
            return BOOL(0);
        }
        let device = unsafe { &*info };
        if !should_auto_pin(device) {
            return BOOL(0);
        }
        let pin = pin_utf16();
        let rc = unsafe {
            BluetoothSendAuthenticationResponse(radio_handle(), device, PCWSTR(pin.as_ptr()))
        };
        let name = device_name(device);
        let addr = addr_ull(device);
        set_auth_note(format!(
            "authLegacy addr={addr:012X} name={name:?} send_rc={rc}"
        ));
        if rc == 0 {
            BOOL(1)
        } else {
            BOOL(0)
        }
    }

    pub fn register_both() -> Result<(isize, isize), TransportError> {
        if let Err(e) = open_local_radio() {
            eprintln!("[authHook] radio open deferred: {e}");
        }

        unsafe {
            let mut reg_ex: isize = 0;
            let rc_ex = BluetoothRegisterForAuthenticationEx(
                None,
                &mut reg_ex,
                Some(on_auth_ex),
                None,
            );
            if rc_ex != 0 || reg_ex == 0 {
                close_local_radio();
                return Err(TransportError::Io(format!(
                    "BluetoothRegisterForAuthenticationEx failed ({rc_ex})"
                )));
            }

            let mut reg_leg: isize = 0;
            let rc_leg = BluetoothRegisterForAuthentication(
                None,
                &mut reg_leg,
                Some(on_auth_legacy),
                None,
            );
            if rc_leg != 0 || reg_leg == 0 {
                let _ = BluetoothUnregisterAuthentication(reg_ex);
                // Legacy optional on some builds — keep Ex alone.
                eprintln!(
                    "[authHook] legacy RegisterForAuthentication failed ({rc_leg}) — Ex only"
                );
                eprintln!("[authHook] AuthenticationEx registered handle={reg_ex}");
                return Ok((reg_ex, 0));
            }

            eprintln!(
                "[authHook] AuthenticationEx+Legacy registered ex={reg_ex} legacy={reg_leg}"
            );
            Ok((reg_ex, reg_leg))
        }
    }

    pub fn unregister(reg_ex: isize, reg_legacy: isize) {
        unsafe {
            if reg_ex != 0 {
                let _ = BluetoothUnregisterAuthentication(reg_ex);
            }
            if reg_legacy != 0 {
                let _ = BluetoothUnregisterAuthentication(reg_legacy);
            }
        }
        close_local_radio();
    }
}
