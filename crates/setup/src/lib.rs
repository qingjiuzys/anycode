//! Shared first-run setup: config readiness, memory presets, workspace layout.

mod cloud_auth;
mod config;
mod memory;
mod quick_auth;
mod status;
mod workspace;

pub use cloud_auth::{
    account_api_url, browser_url_for_device_link, cloud_session_path, gateway_url, link_device,
    poll_device_link, portal_login_url_for_device, portal_url, read_access_token,
    read_cloud_session, start_device_link, try_poll_device_link, write_cloud_session, CloudSession,
    DeviceLinkStart, DEVICE_LINK_REDIRECT_URI,
};
pub use config::has_usable_model_config;
pub use memory::{apply_memory_preset, memory_preset_from_label, MemorySetupPreset};
pub use quick_auth::{quick_auth_presets, QuickAuthChoice, QUICK_AUTH_CHOICES};
pub use status::{
    build_setup_status, load_setup_status, SetupStatus, SetupStepId, SetupStepStatus,
};
pub use workspace::{ensure_layout, workspace_root};
