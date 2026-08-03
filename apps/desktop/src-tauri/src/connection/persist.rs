//! Persist known RFCOMM devices (BD_ADDR is canonical) in app data.
//!
//! Store format is multi-device ready (`rfcomm_devices.json`): a device list
//! plus one active address. Stufe A only ever connects the active device;
//! the list exists so a later device-memory UI needs no second migration.
//! Legacy single-target files (`rfcomm_known_target.json`) migrate on load.

use crate::transport::rfcomm::target::RfcommTarget;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FILE_NAME: &str = "rfcomm_devices.json";
const LEGACY_FILE_NAME: &str = "rfcomm_known_target.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownDevice {
    #[serde(flatten)]
    pub target: RfcommTarget,
    /// Unix seconds of the last successful connect (None = never linked).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<u64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStore {
    /// BD_ADDR of the device the app connects to (must be in `devices`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_addr: Option<u64>,
    #[serde(default)]
    pub devices: Vec<KnownDevice>,
}

impl DeviceStore {
    pub fn active_target(&self) -> Option<RfcommTarget> {
        let addr = self.active_addr?;
        self.devices
            .iter()
            .find(|d| d.target.bt_addr == addr)
            .map(|d| d.target.clone())
    }

    /// Set `target` active and upsert it into the device list.
    pub fn set_active(&mut self, target: &RfcommTarget, connected_at: Option<u64>) {
        self.active_addr = Some(target.bt_addr);
        if let Some(existing) = self
            .devices
            .iter_mut()
            .find(|d| d.target.bt_addr == target.bt_addr)
        {
            existing.target = target.clone();
            if connected_at.is_some() {
                existing.last_connected_at = connected_at;
            }
        } else {
            self.devices.push(KnownDevice {
                target: target.clone(),
                last_connected_at: connected_at,
            });
        }
    }

    /// Forget the active device (removes it from the list entirely).
    pub fn clear_active(&mut self) {
        if let Some(addr) = self.active_addr.take() {
            self.devices.retain(|d| d.target.bt_addr != addr);
        }
    }
}

pub fn device_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

fn legacy_target_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LEGACY_FILE_NAME)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Load the device store; migrates a legacy single-target file once.
pub fn load_device_store(data_dir: &Path) -> DeviceStore {
    let path = device_store_path(data_dir);
    if let Ok(bytes) = fs::read(&path) {
        if let Ok(store) = serde_json::from_slice::<DeviceStore>(&bytes) {
            return store;
        }
    }
    migrate_legacy(data_dir).unwrap_or_default()
}

/// One-shot migration: legacy known target becomes the active device.
fn migrate_legacy(data_dir: &Path) -> Option<DeviceStore> {
    let legacy = legacy_target_path(data_dir);
    let bytes = fs::read(&legacy).ok()?;
    let target = serde_json::from_slice::<RfcommTarget>(&bytes).ok()?;
    let mut store = DeviceStore::default();
    // Legacy file existed → the device was successfully linked at least once.
    store.set_active(&target, Some(now_unix()));
    if save_device_store(data_dir, &store).is_ok() {
        let _ = fs::remove_file(&legacy);
    }
    Some(store)
}

pub fn save_device_store(data_dir: &Path, store: &DeviceStore) -> Result<(), String> {
    let path = device_store_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Active device as `RfcommTarget` (owner/startup contract, unchanged shape).
pub fn load_known_target(data_dir: &Path) -> Option<RfcommTarget> {
    load_device_store(data_dir).active_target()
}

/// Persist `target` as active device (upsert + last-connected timestamp).
pub fn save_known_target(data_dir: &Path, target: &RfcommTarget) -> Result<(), String> {
    let mut store = load_device_store(data_dir);
    store.set_active(target, Some(now_unix()));
    save_device_store(data_dir, &store)
}

/// „Gerät vergessen“: drop the active device from the store.
pub fn clear_known_target(data_dir: &Path) -> Result<(), String> {
    let mut store = load_device_store(data_dir);
    store.clear_active();
    save_device_store(data_dir, &store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::rfcomm::target::SPP_SERVICE_UUID;

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rpa_persist_{tag}_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn target(addr: u64, name: &str) -> RfcommTarget {
        RfcommTarget {
            bt_addr: addr,
            display_name: name.to_string(),
            service_uuid: SPP_SERVICE_UUID.to_string(),
            rfcomm_channel: None,
            com_port: None,
        }
    }

    #[test]
    fn save_load_roundtrip_keeps_active_target() {
        let dir = tmp_dir("roundtrip");
        let t = target(0x0018DA070564, "KT RDT ZIE 1");
        save_known_target(&dir, &t).unwrap();
        assert_eq!(load_known_target(&dir), Some(t));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_legacy_single_target_file() {
        let dir = tmp_dir("legacy");
        let t = target(0x0018DA070564, "KT RDT ZIE 1");
        fs::write(
            legacy_target_path(&dir),
            serde_json::to_vec_pretty(&t).unwrap(),
        )
        .unwrap();

        assert_eq!(load_known_target(&dir), Some(t.clone()));
        // Legacy file is gone, new store carries the device.
        assert!(!legacy_target_path(&dir).exists());
        let store = load_device_store(&dir);
        assert_eq!(store.active_addr, Some(t.bt_addr));
        assert_eq!(store.devices.len(), 1);
        assert!(store.devices[0].last_connected_at.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn switching_active_keeps_previous_device_in_list() {
        let dir = tmp_dir("switch");
        let a = target(0xAAAA_AAAA_AAAA, "KT RDT ZIE 1");
        let b = target(0xBBBB_BBBB_BBBB, "KT RDT ZIE 2");
        save_known_target(&dir, &a).unwrap();
        save_known_target(&dir, &b).unwrap();

        let store = load_device_store(&dir);
        assert_eq!(store.active_addr, Some(b.bt_addr));
        assert_eq!(store.devices.len(), 2);
        assert_eq!(load_known_target(&dir), Some(b));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_forgets_only_active_device() {
        let dir = tmp_dir("clear");
        let a = target(0xAAAA_AAAA_AAAA, "KT RDT ZIE 1");
        let b = target(0xBBBB_BBBB_BBBB, "KT RDT ZIE 2");
        save_known_target(&dir, &a).unwrap();
        save_known_target(&dir, &b).unwrap();

        clear_known_target(&dir).unwrap();
        assert_eq!(load_known_target(&dir), None);
        let store = load_device_store(&dir);
        assert_eq!(store.active_addr, None);
        assert_eq!(store.devices.len(), 1);
        assert_eq!(store.devices[0].target.bt_addr, a.bt_addr);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_files_yield_empty_store() {
        let dir = tmp_dir("empty");
        assert_eq!(load_known_target(&dir), None);
        clear_known_target(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
