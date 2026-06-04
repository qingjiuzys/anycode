use crate::api::state::AppState;
use crate::cron_ledger;
use crate::schema::{
    CreateSessionRequest, CronJobRecord, CronRunRecord, HealthResponse, InsertEventRequest,
    LocalServiceRecord, UpsertProjectRequest,
};
use crate::skills_scan;
use async_stream::stream;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json,
    },
};
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;

#[derive(Deserialize)]
pub struct LimitQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct EventsQuery {
    pub after: Option<String>,
    #[serde(default = "default_events_limit")]
    pub limit: i64,
    /// Filter by `project_events.event_type` (e.g. `tool_call_end`, `workflow_step`).
    pub event_type: Option<String>,
    pub severity: Option<String>,
    /// Substring match against title and body.
    pub q: Option<String>,
}

fn default_events_limit() -> i64 {
    200
}

#[derive(Deserialize)]
pub struct ArtifactsQuery {
    pub kind: Option<String>,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub trust_level: Option<String>,
    #[serde(default)]
    pub unverified_only: bool,
    #[serde(default)]
    pub blocked_session_only: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct SessionsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Comma-separated session kinds: `workflow`, `cron`, …
    pub kind: Option<String>,
    pub status: Option<String>,
    pub trusted_status: Option<String>,
    pub project_id: Option<String>,
    /// When true, only sessions with a `budget_exceeded` event.
    pub budget_exceeded: Option<bool>,
}

mod agents;
mod assets;
mod auth;
mod chat_util;
mod connectors;
mod conversations;
mod events;
mod gates;
mod governance;
mod knowledge;
mod model_catalog;
mod models;
mod operations;
mod projects;
mod reports;
mod sessions;
mod settings;
mod system;

pub use agents::*;
pub use assets::*;
pub use auth::*;
pub use connectors::*;
pub use conversations::*;
pub use events::*;
pub use gates::*;
pub use governance::*;
pub use knowledge::*;
pub use model_catalog::*;
pub use models::*;
pub use operations::*;
pub use projects::*;
pub use reports::*;
pub use sessions::*;
pub use settings::*;
pub use system::*;
