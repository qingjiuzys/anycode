pub mod api;
pub mod asset_index;
pub mod assets;
pub mod audit;
pub mod auth_session;
pub mod bootstrap;
pub mod browser_connector;
pub mod config_patch;
pub mod connector_health;
pub mod connectors;
pub mod control;
pub mod cron_ledger;
pub mod data_health;
pub mod db;
pub mod db_ops;
pub mod embedded_ui;
pub mod events;
pub mod governance;
pub mod ipc;
pub mod llm_probe;
pub mod local_service;
pub mod mcp_config;
pub mod media_defaults;
pub mod memory_ops;
pub mod model_identity;
pub mod notifications;
pub mod notify;
pub mod observability;
pub mod preferences;
pub mod project_knowledge;
pub mod project_root;
pub mod project_skills;
pub mod recorder;
pub mod report;
pub mod report_archive;
pub mod runtime_config;
pub mod schema;
pub mod search;
pub mod server;
pub mod skill_market;
pub mod skill_meta;
pub mod skill_suggestions;
pub mod skills_scan;
pub mod static_ui;
pub mod tokens;
pub mod workspace_index;
pub mod workspace_scan;

pub use control::{gate_runner, task_trigger};
pub use governance::{automation_policy, security_events, service_governance, skills_governance};
pub use ipc::{approval_ipc, cancel_ipc, question_ipc};
pub use observability::{event_tier, execution_log};
pub use observability::{
    ingest, log_parser, metrics, session_replay, session_trace, session_transcript,
    transcript_cache,
};

#[cfg(test)]
mod test_util;

pub use db::DashboardDb;
pub use db_ops::{backup_db, db_operations};
pub use events::{EventBus, EventSink};
pub use log_parser::parse_line;
pub use preferences::{load_preferences, preferences_path, restart_command, save_preferences};
pub use recorder::{DashboardRecorder, RunSessionKind};
pub use server::{default_db_path, run, DashboardConfig};
pub use service_governance::{is_loopback_host, run_doctor_checks, suggest_backup_path};
pub use static_ui::{discover_ui_dist, ui_available};
pub use tokens::{create_token, list_tokens, revoke_token, token_count_active};
pub use workspace_index::{
    collect_scan_workspace_paths, discover_paths_from_sessions, load_workspace_paths,
    workspace_index_path,
};
