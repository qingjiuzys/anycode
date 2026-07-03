use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LockHolder {
    Idle,
    Agent,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTabInfo {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub url: String,
    pub title: String,
    pub lock: LockHolder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserViewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScreenshot {
    pub image_base64: String,
    pub viewport: BrowserViewport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshot {
    pub url: String,
    pub title: String,
    pub yaml: String,
}

#[derive(Debug, Clone)]
pub struct ScreencastFrame {
    pub image_base64: String,
    pub metadata: ScreencastMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreencastMetadata {
    pub offset_top: i64,
    pub page_scale_factor: f64,
    pub device_width: i64,
    pub device_height: i64,
    pub scroll_offset_x: i64,
    pub scroll_offset_y: i64,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionInfo {
    pub session_id: String,
    pub project_id: String,
    pub conversation_id: Option<String>,
}
