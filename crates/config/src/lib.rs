//! Shared configuration: `~/.anycode/config.json` schema, load/save, and runtime `Config`.

mod load;
mod schema;
mod user_config;
pub mod workspace;

pub use load::{
    apply_channel_self_hosted_security, load_config, load_config_for_session, load_runtime_config,
    security_wants_interactive_approval_callback, LoadOpts,
};
pub use schema::*;
pub use user_config::*;
pub use workspace::{
    apply_project_overlays, canonical_root_string, ensure_layout, root as workspace_root,
    touch_project_dir, update_project_metadata, WorkspaceProject,
};
