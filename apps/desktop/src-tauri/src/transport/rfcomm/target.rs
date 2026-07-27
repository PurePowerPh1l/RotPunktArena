//! Known RFCOMM target identity (BD_ADDR is canonical).

use serde::{Deserialize, Serialize};

/// Standard Serial Port Profile UUID.
pub const SPP_SERVICE_UUID: &str = "00001101-0000-1000-8000-00805F9B34FB";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RfcommTarget {
    /// 48-bit Bluetooth address as integer.
    pub bt_addr: u64,
    pub display_name: String,
    /// SPP service UUID string (canonical form).
    pub service_uuid: String,
    /// Cached RFCOMM channel from last successful SDP/connect (legacy / unused on COM path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rfcomm_channel: Option<u32>,
    /// Windows Virtual COM for this BD_ADDR (e.g. COM4) — preferred hardware path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub com_port: Option<String>,
}

impl RfcommTarget {
    pub fn addr_hex(&self) -> String {
        format!("{:012X}", self.bt_addr & 0xFFFF_FFFF_FFFF)
    }

    pub fn summary(&self) -> TargetSummary {
        TargetSummary {
            bt_addr_hex: self.addr_hex(),
            display_name: self.display_name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSummary {
    pub bt_addr_hex: String,
    pub display_name: String,
}
