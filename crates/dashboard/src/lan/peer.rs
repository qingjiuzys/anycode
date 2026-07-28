//! Discovered LAN peer metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanPeer {
    pub instance_id: String,
    pub device_name: String,
    pub host: String,
    pub lan_port: u16,
    pub version: String,
    #[serde(default = "Utc::now")]
    pub last_seen: DateTime<Utc>,
}

impl LanPeer {
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.lan_port)
    }
}
