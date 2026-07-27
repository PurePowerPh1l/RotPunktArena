//! Persist known RFCOMM target (BD_ADDR) in app data.

use crate::transport::rfcomm::target::RfcommTarget;
use std::fs;
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "rfcomm_known_target.json";

pub fn known_target_path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load_known_target(data_dir: &Path) -> Option<RfcommTarget> {
    let path = known_target_path(data_dir);
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_known_target(data_dir: &Path, target: &RfcommTarget) -> Result<(), String> {
    let path = known_target_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(target).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn clear_known_target(data_dir: &Path) -> Result<(), String> {
    let path = known_target_path(data_dir);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
