//! Virtual COM (BT-SPP) link — uses Windows' existing pairing, no Winsock RFCOMM connect.
//!
//! Opening the outgoing COMx for a paired RedDot lets the OS stack authenticate
//! via the existing bond. Native AF_BTH connect() re-triggers pairing UI.

use crate::protocol::BAUD_RATE;
use crate::transport::rfcomm::error::TransportError;
use crate::transport::rfcomm::ByteTransport;
use crate::transport::serial::{self, list_ports};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::time::Duration;

pub struct SerialLink {
    port_name: String,
    port: Box<dyn SerialPort>,
}

impl SerialLink {
    pub fn open(port_name: &str, timeout: Duration) -> Result<Self, TransportError> {
        let port = serialport::new(port_name, BAUD_RATE)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(timeout.min(Duration::from_millis(200)).max(Duration::from_millis(50)))
            .open()
            .map_err(|e| TransportError::Io(format!("COM {port_name}: {e}")))?;
        Ok(Self {
            port_name: port_name.to_string(),
            port,
        })
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }
}

impl ByteTransport for SerialLink {
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        self.port
            .set_timeout(timeout)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        match self.port.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(TransportError::Timeout),
            Err(e) => Err(TransportError::Io(e.to_string())),
        }
    }

    fn write_all(&mut self, data: &[u8], _timeout: Duration) -> Result<(), TransportError> {
        self.port
            .write_all(data)
            .map_err(|e| TransportError::Io(e.to_string()))?;
        self.port
            .flush()
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), TransportError> {
        // Dropping the port closes it.
        Ok(())
    }
}

/// Resolve outgoing SPP COM for a BD_ADDR (Windows PnP), else ENQ auto-detect.
pub fn resolve_spp_com(bt_addr: u64, preferred: Option<&str>) -> Option<String> {
    let ports = list_ports();
    if let Some(p) = preferred {
        if ports.iter().any(|x| x.eq_ignore_ascii_case(p)) {
            return Some(p.to_string());
        }
    }
    if let Some(p) = find_com_for_bt_addr(bt_addr) {
        return Some(p);
    }
    // Last resort: ENQ probe (opens briefly — OS uses existing bond, no PIN UI).
    serial::auto_detect(preferred)
}

#[cfg(windows)]
fn find_com_for_bt_addr(bt_addr: u64) -> Option<String> {
    pnp_com::find_com_for_bt_addr(bt_addr)
}

#[cfg(not(windows))]
fn find_com_for_bt_addr(_bt_addr: u64) -> Option<String> {
    None
}

#[cfg(windows)]
mod pnp_com {
    use std::mem::{size_of, zeroed};
    use std::ptr;

    #[repr(C)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
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

    pub fn find_com_for_bt_addr(bt_addr: u64) -> Option<String> {
        let needle = format!("DEV_{:012X}", bt_addr & 0xFFFF_FFFF_FFFF);
        unsafe {
            let enum_bthenum: Vec<u16> = "BTHENUM\0".encode_utf16().collect();
            let devs = SetupDiGetClassDevsW(
                ptr::null(),
                enum_bthenum.as_ptr(),
                ptr::null_mut(),
                DIGCF_PRESENT | DIGCF_ALLCLASSES,
            );
            if devs.is_null() || devs == (-1isize as *mut std::ffi::c_void) {
                return None;
            }
            let mut info: SP_DEVINFO_DATA = zeroed();
            info.cb_size = size_of::<SP_DEVINFO_DATA>() as u32;
            let mut idx = 0u32;
            let mut found: Option<String> = None;
            while SetupDiEnumDeviceInfo(devs, idx, &mut info) != 0 {
                idx += 1;
                let mut id_buf = vec![0u16; 512];
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
                let id = String::from_utf16_lossy(
                    &id_buf[..id_buf.iter().position(|&c| c == 0).unwrap_or(0)],
                );
                if !id.to_uppercase().contains(&needle) {
                    continue;
                }
                // Friendly name often "Standard Serial over Bluetooth link (COM4)"
                if let Some(name) = reg_prop_string(devs, &info, SPDRP_FRIENDLYNAME) {
                    if let Some(com) = extract_com(&name) {
                        found = Some(com);
                        break;
                    }
                }
            }
            let _ = SetupDiDestroyDeviceInfoList(devs);
            found
        }
    }

    unsafe fn reg_prop_string(
        devs: *mut std::ffi::c_void,
        info: *const SP_DEVINFO_DATA,
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

    fn extract_com(friendly: &str) -> Option<String> {
        // "(COM4)" or "COM4"
        let upper = friendly.to_uppercase();
        if let Some(i) = upper.find("COM") {
            let rest = &upper[i..];
            let digits: String = rest
                .chars()
                .skip(3)
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if !digits.is_empty() {
                return Some(format!("COM{digits}"));
            }
        }
        None
    }
}
