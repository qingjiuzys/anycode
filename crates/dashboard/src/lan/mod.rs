//! LAN colleague discovery and project/session handoff.

mod bundle;
mod discovery;
mod handoff;
mod instance;
mod listener;
mod peer;
mod security;

pub use bundle::{export_bundle, import_bundle, BundleExportOptions, ImportOptions};
pub use discovery::{primary_lan_ip, spawn_discovery, DEFAULT_LAN_PORT, MDNS_SERVICE_TYPE};
pub use handoff::{
    HandoffApprovedNotice, HandoffDirection, HandoffKind, HandoffParty, HandoffRecord,
    HandoffState, IncomingHandoffRequest, OutgoingHandoffStatus,
};
pub use instance::{load_or_create_instance, save_instance, LanInstance, LanSettings};
pub use listener::{spawn_lan_listener, LanListenerState};
pub use peer::LanPeer;
pub use security::is_private_ip;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::RwLock as AsyncRwLock;

/// Shared LAN hub attached to dashboard `AppState`.
#[derive(Clone)]
pub struct LanHub {
    pub instance: LanInstance,
    pub settings: Arc<AsyncRwLock<LanSettings>>,
    pub peers: Arc<RwLock<HashMap<String, LanPeer>>>,
    pub handoffs: Arc<AsyncRwLock<HashMap<String, HandoffRecord>>>,
    pub version: String,
    pub data_dir: std::path::PathBuf,
}

impl LanHub {
    pub fn new(version: String, data_dir: std::path::PathBuf) -> Self {
        let instance = load_or_create_instance(&data_dir);
        let settings = LanSettings::load(&data_dir);
        Self {
            instance,
            settings: Arc::new(AsyncRwLock::new(settings)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            handoffs: Arc::new(AsyncRwLock::new(HashMap::new())),
            version,
            data_dir,
        }
    }

    pub async fn settings_snapshot(&self) -> LanSettings {
        self.settings.read().await.clone()
    }

    pub fn list_peers(&self) -> Vec<LanPeer> {
        let map = self.peers.read().unwrap_or_else(|e| e.into_inner());
        let self_id = self.instance.instance_id.clone();
        let mut peers: Vec<LanPeer> = map
            .values()
            .filter(|p| p.instance_id != self_id)
            .cloned()
            .collect();
        peers.sort_by(|a, b| a.device_name.cmp(&b.device_name));
        peers
    }

    pub fn upsert_peer(&self, peer: LanPeer) {
        if peer.instance_id == self.instance.instance_id {
            return;
        }
        if let Ok(mut map) = self.peers.write() {
            map.insert(peer.instance_id.clone(), peer);
        }
    }

    pub fn remove_stale_peers(&self, max_age_secs: i64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(max_age_secs);
        if let Ok(mut map) = self.peers.write() {
            map.retain(|_, p| p.last_seen >= cutoff);
        }
    }

    /// Load optional `dev_peers.json` for same-machine testing (mDNS cannot register twice).
    pub fn refresh_dev_peers(&self) {
        let path = self.data_dir.join("dev_peers.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(peers) = serde_json::from_str::<Vec<LanPeer>>(&text) else {
            return;
        };
        for peer in peers {
            self.upsert_peer(peer);
        }
    }
}

pub fn lan_data_dir() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("ANYCODE_LAN_DATA_DIR") {
        if !path.trim().is_empty() {
            return std::path::PathBuf::from(path);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".anycode")
        .join("lan")
}

pub fn lan_enabled() -> bool {
    !std::env::var("ANYCODE_LAN_DISCOVERY")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}
