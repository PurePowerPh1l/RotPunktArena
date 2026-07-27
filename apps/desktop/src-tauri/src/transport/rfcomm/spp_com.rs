//! Best-effort release of the Windows Virtual SPP COM port for a BD_ADDR.
//!
//! Native RFCOMM (channel 1) and "Standard Serial over Bluetooth link (COMx)"
//! share the same RFCOMM channel. Leaving the COM enabled often yields
//! `WSAEADDRINUSE` / AuthEx spam. We disable the PnP node before connect and
//! re-enable on forget / shutdown.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

fn disabled() -> &'static Mutex<HashSet<String>> {
    static DISABLED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    DISABLED.get_or_init(|| Mutex::new(HashSet::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SppComAction {
    /// No matching COM / already free.
    None,
    /// We disabled the PnP node for this address.
    Disabled { instance_id: String, com: String },
    /// Port found but disable failed (permissions / in use).
    Busy { com: String, detail: String },
}

/// Disable the SPP COM for `bt_addr` if present and still enabled.
pub fn release_channel_for(bt_addr: u64) -> SppComAction {
    #[cfg(windows)]
    {
        win::release_channel_for(bt_addr)
    }
    #[cfg(not(windows))]
    {
        let _ = bt_addr;
        SppComAction::None
    }
}

/// Re-enable every COM we previously disabled (forget / shutdown).
pub fn restore_all() {
    #[cfg(windows)]
    {
        win::restore_all();
    }
}

/// Re-enable COM nodes for one BD_ADDR (if we disabled them).
pub fn restore_for(bt_addr: u64) {
    #[cfg(windows)]
    {
        win::restore_for(bt_addr);
    }
    #[cfg(not(windows))]
    {
        let _ = bt_addr;
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use ::windows::core::PCWSTR;
    use ::windows::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW, CM_Disable_DevNode,
        CM_Enable_DevNode, CM_Locate_DevNodeW, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
        DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SPDRP_FRIENDLYNAME, SP_DEVINFO_DATA,
    };

    struct ComNode {
        instance_id: String,
        com: String,
        dev_inst: u32,
    }

    fn extract_com(friendly: &str) -> Option<String> {
        let upper = friendly.to_uppercase();
        let i = upper.find("COM")?;
        let digits: String = upper[i..]
            .chars()
            .skip(3)
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            None
        } else {
            Some(format!("COM{digits}"))
        }
    }

    unsafe fn reg_prop_string(devs: HDEVINFO, info: &SP_DEVINFO_DATA) -> Option<String> {
        let mut reg_type = 0u32;
        let mut required = 0u32;
        let mut buf = vec![0u8; 512];
        if SetupDiGetDeviceRegistryPropertyW(
            devs,
            info,
            SPDRP_FRIENDLYNAME,
            Some(&mut reg_type),
            Some(&mut buf),
            Some(&mut required),
        )
        .is_err()
        {
            if required > buf.len() as u32 {
                buf.resize(required as usize, 0);
                if SetupDiGetDeviceRegistryPropertyW(
                    devs,
                    info,
                    SPDRP_FRIENDLYNAME,
                    Some(&mut reg_type),
                    Some(&mut buf),
                    Some(&mut required),
                )
                .is_err()
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

    fn find_com_nodes(bt_addr: u64) -> Vec<ComNode> {
        let needle = format!("DEV_{:012X}", bt_addr & 0xFFFF_FFFF_FFFF);
        let mut out = Vec::new();
        unsafe {
            let enum_bthenum: Vec<u16> = "BTHENUM\0".encode_utf16().collect();
            let Ok(devs) = SetupDiGetClassDevsW(
                None,
                PCWSTR(enum_bthenum.as_ptr()),
                None,
                DIGCF_PRESENT | DIGCF_ALLCLASSES,
            ) else {
                return out;
            };
            if devs.is_invalid() {
                return out;
            }
            let mut info = SP_DEVINFO_DATA::default();
            info.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
            let mut idx = 0u32;
            while SetupDiEnumDeviceInfo(devs, idx, &mut info).is_ok() {
                idx += 1;
                let mut id_buf = vec![0u16; 512];
                let mut required = 0u32;
                if SetupDiGetDeviceInstanceIdW(
                    devs,
                    &info,
                    Some(&mut id_buf),
                    Some(&mut required),
                )
                .is_err()
                {
                    continue;
                }
                let id = String::from_utf16_lossy(
                    &id_buf[..id_buf.iter().position(|&c| c == 0).unwrap_or(0)],
                );
                if !id.to_uppercase().contains(&needle) {
                    continue;
                }
                let friendly = reg_prop_string(devs, &info).unwrap_or_default();
                let Some(com) = extract_com(&friendly) else {
                    continue;
                };
                out.push(ComNode {
                    instance_id: id,
                    com,
                    dev_inst: info.DevInst,
                });
            }
            let _ = SetupDiDestroyDeviceInfoList(devs);
        }
        out
    }

    fn enable_instance(instance_id: &str) -> Result<(), String> {
        unsafe {
            let wide: Vec<u16> = instance_id
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut dev_inst = 0u32;
            let cr = CM_Locate_DevNodeW(
                &mut dev_inst,
                PCWSTR(wide.as_ptr()),
                CM_LOCATE_DEVNODE_NORMAL,
            );
            if cr != CR_SUCCESS {
                return Err(format!("CM_Locate_DevNodeW={:?}", cr));
            }
            let cr = CM_Enable_DevNode(dev_inst, 0);
            if cr != CR_SUCCESS {
                return Err(format!("CM_Enable_DevNode={:?}", cr));
            }
            Ok(())
        }
    }

    pub fn release_channel_for(bt_addr: u64) -> SppComAction {
        let nodes = find_com_nodes(bt_addr);
        if nodes.is_empty() {
            return SppComAction::None;
        }
        let mut last_busy: Option<(String, String)> = None;
        let mut did_disable: Option<(String, String)> = None;
        for node in nodes {
            {
                let g = disabled().lock().unwrap();
                if g.contains(&node.instance_id) {
                    did_disable = Some((node.instance_id, node.com));
                    continue;
                }
            }
            unsafe {
                let cr = CM_Disable_DevNode(node.dev_inst, 0);
                if cr == CR_SUCCESS {
                    disabled().lock().unwrap().insert(node.instance_id.clone());
                    did_disable = Some((node.instance_id, node.com));
                } else {
                    last_busy = Some((
                        node.com,
                        format!("CM_Disable_DevNode={:?}", cr),
                    ));
                }
            }
        }
        if let Some((instance_id, com)) = did_disable {
            return SppComAction::Disabled { instance_id, com };
        }
        if let Some((com, detail)) = last_busy {
            return SppComAction::Busy { com, detail };
        }
        SppComAction::None
    }

    pub fn restore_all() {
        let ids: Vec<String> = disabled().lock().unwrap().drain().collect();
        for id in ids {
            let _ = enable_instance(&id);
        }
    }

    pub fn restore_for(bt_addr: u64) {
        let needle = format!("DEV_{:012X}", bt_addr & 0xFFFF_FFFF_FFFF);
        let ids: Vec<String> = {
            let mut g = disabled().lock().unwrap();
            let take: Vec<String> = g
                .iter()
                .filter(|id| id.to_uppercase().contains(&needle))
                .cloned()
                .collect();
            for id in &take {
                g.remove(id);
            }
            take
        };
        for id in ids {
            let _ = enable_instance(&id);
        }
        for node in find_com_nodes(bt_addr) {
            let _ = enable_instance(&node.instance_id);
        }
    }
}
