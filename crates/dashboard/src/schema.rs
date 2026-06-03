//! API DTOs for the Digital Workbench.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const LOCAL_ORG_ID: &str = "org_local";
pub const LOCAL_USER_ID: &str = "user_local";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
    pub db_path: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub status: String,
    pub trust_score: f64,
    pub sessions_count: i64,
    pub artifacts_count: i64,
    pub updated_at: String,
    #[serde(default)]
    pub root_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFacetsResponse {
    pub status: Vec<LabelCount>,
    pub trusted_status: Vec<LabelCount>,
    pub kind: Vec<LabelCount>,
    pub pending_approval_total: i64,
    pub budget_exceeded_7d: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptBlock {
    pub id: String,
    pub block_type: String,
    pub at: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub collapsible: bool,
    #[serde(default)]
    pub default_collapsed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTranscriptResponse {
    pub schema_version: u32,
    pub session_id: String,
    pub blocks: Vec<TranscriptBlock>,
    pub lifecycle: Vec<TranscriptBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub description: String,
    pub business_goal: String,
    pub status: String,
    pub trust_score: f64,
    pub automation_level: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub kind: String,
    pub task_id: Option<String>,
    pub title: String,
    pub status: String,
    pub trusted_status: String,
    pub agent_type: String,
    pub model: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWithProject {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub kind: String,
    pub task_id: Option<String>,
    pub title: String,
    pub status: String,
    pub trusted_status: String,
    pub agent_type: String,
    pub model: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub kind: String,
    pub task_id: Option<String>,
    pub title: String,
    pub prompt_preview: String,
    pub status: String,
    pub trusted_status: String,
    pub agent_type: String,
    pub model: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: String,
    pub metadata_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub projects_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewStats {
    pub projects_count: i64,
    pub sessions_total: i64,
    pub sessions_running: i64,
    pub sessions_blocked: i64,
    pub sessions_budget_exceeded: i64,
    pub artifacts_count: i64,
    pub skills_count: i64,
    pub gates_failed: i64,
    pub events_last_hour: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEventRecord {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityActivitySummary {
    pub denied_total: i64,
    pub pending_total: i64,
    pub recent: Vec<SecurityEventRecord>,
    pub read_only: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEvent {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub session_id: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEvent {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub payload: Value,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelCount {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatsFailure {
    pub id: String,
    pub title: String,
    pub event_type: String,
    pub occurred_at: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub event_types: Vec<LabelCount>,
    pub severities: Vec<LabelCount>,
    pub session_statuses: Vec<LabelCount>,
    pub gate_statuses: Vec<LabelCount>,
    pub recent_failures: Vec<ProjectStatsFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRecord {
    pub id: String,
    pub name: String,
    pub status: String,
    pub required: bool,
    pub output_excerpt: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub title: String,
    pub trust_level: String,
    pub verified_by_gate_id: Option<String>,
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_by_gate_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_trusted_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchHit {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResults {
    pub query: String,
    pub projects: Vec<SearchHit>,
    pub sessions: Vec<SearchHit>,
    pub events: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertEventRequest {
    pub project_id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub event_type: String,
    pub severity: Option<String>,
    pub title: String,
    pub body: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertProjectRequest {
    pub root_path: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// When true, create `root_path` on disk if it does not exist yet.
    pub create_root: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub project_id: String,
    pub kind: String,
    pub task_id: Option<String>,
    pub title: String,
    pub prompt_preview: Option<String>,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartConversationRequest {
    #[serde(default)]
    pub title: Option<String>,
    pub prompt: String,
    #[serde(default = "default_start_kind")]
    pub kind: String,
    pub goal: Option<String>,
    pub agent: Option<String>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
}

fn default_start_kind() -> String {
    "run".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendConversationMessageRequest {
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRunRecord {
    pub job_id: String,
    pub session_id: String,
    pub fired_at: String,
    pub status: String,
    pub detail: String,
    pub line_no: usize,
    pub dashboard_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobRecord {
    pub id: String,
    pub schedule: String,
    pub command: String,
    pub session_id: Option<String>,
    pub failure_destination: Option<String>,
    pub tool_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsageStat {
    pub agent_type: String,
    pub model: String,
    pub sessions_count: i64,
    pub last_started_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalServiceRecord {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub auth_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub sessions: i64,
    pub events: i64,
    pub failed_gates: i64,
    pub artifacts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSourceCounts {
    pub sessions: i64,
    pub events: i64,
    pub gates: i64,
    pub artifacts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDocument {
    pub scope: String,
    pub id: String,
    pub title: String,
    pub format: String,
    pub generated_at: String,
    pub trusted_status: String,
    pub markdown: String,
    pub summary: ReportSummary,
    pub source_counts: ReportSourceCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub actor: String,
    pub action: String,
    pub risk: String,
    pub detail: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    pub mode: String,
    pub host_binding: String,
    pub remote_access_allowed: bool,
    pub write_actions_allowed: bool,
    pub safe_actions: Vec<String>,
    pub blocked_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub message: String,
    pub count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHealth {
    pub status: String,
    pub db_path: String,
    pub db_size_bytes: u64,
    pub generated_at: String,
    pub checks: Vec<HealthCheckItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusDetail {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub auth_mode: String,
    pub version: String,
    pub pid: Option<u32>,
    pub started_at: String,
    pub db_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_dist: Option<String>,
    pub ui_dist_present: bool,
    pub sse_subscribers: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_at: Option<String>,
    pub loopback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub status: String,
    pub generated_at: String,
    pub checks: Vec<DoctorCheck>,
    #[serde(default)]
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenRecord {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryReadiness {
    pub status: String,
    pub blocked_sessions: i64,
    pub failed_required_gates: i64,
    pub unverified_artifacts: i64,
    pub stale_running_sessions: i64,
    pub running_sessions: i64,
    pub projects: Vec<ProjectReadinessItem>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectReadinessItem {
    pub project_id: String,
    pub project_name: String,
    pub readiness_score: i64,
    pub blocked_sessions: i64,
    pub failed_gates: i64,
    pub unverified_artifacts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    pub project_id: String,
    pub sessions_total: i64,
    pub sessions_completed: i64,
    pub blocked_sessions: i64,
    pub failed_required_gates: i64,
    pub unverified_artifacts: i64,
    pub stale_running_sessions: i64,
    pub events_7d: i64,
    pub gate_pass_rate: f64,
    pub session_success_rate: f64,
    pub readiness_score: i64,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationPolicyRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub enabled: bool,
    pub policy_type: String,
    pub config: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVersionRecord {
    pub id: String,
    pub artifact_id: String,
    pub hash: String,
    pub size_bytes: i64,
    pub indexed_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLinkRecord {
    pub id: String,
    pub artifact_id: String,
    pub link_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDetail {
    pub artifact: ArtifactRecord,
    pub versions: Vec<ArtifactVersionRecord>,
    pub links: Vec<ArtifactLinkRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProjectLink {
    pub project_id: String,
    pub project_name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetailRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub permissions: Value,
    pub projects_count: i64,
    pub projects: Vec<SkillProjectLink>,
    pub recent_runs: Vec<SkillRunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRunRecord {
    pub id: String,
    pub skill_id: String,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub status: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPolicyRecord {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub event_type: String,
    pub channel: String,
    pub enabled: bool,
    pub config: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorRecord {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub source_type: String,
    pub name: String,
    pub enabled: bool,
    pub config_summary: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    pub version: i64,
    pub name: String,
    pub applied_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTableStat {
    pub name: String,
    pub row_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOperations {
    pub db_path: String,
    pub db_size_bytes: u64,
    pub migrations: Vec<MigrationInfo>,
    pub tables: Vec<DbTableStat>,
    pub backup_suggestion: String,
    pub growth_warnings: Vec<String>,
    pub health_status: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedProjectRow {
    pub id: String,
    pub name: String,
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneStaleProjectsReport {
    pub dry_run: bool,
    pub removed: Vec<PrunedProjectRow>,
    pub kept: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryFailure {
    pub event_id: String,
    pub title: String,
    pub event_type: String,
    pub occurred_at: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub event_id: String,
    pub title: String,
    pub body: String,
    pub occurred_at: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracePhaseSummary {
    pub phase: String,
    pub count: i64,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReplaySummary {
    pub session_id: String,
    pub project_id: String,
    pub project_name: String,
    pub title: String,
    pub status: String,
    pub trusted_status: String,
    pub kind: String,
    pub failed_gates: Vec<GateRecord>,
    pub last_error: Option<SessionSummaryFailure>,
    pub artifacts: Vec<ArtifactRecord>,
    pub recent_events: Vec<ProjectEvent>,
    pub report_artifacts: Vec<ArtifactRecord>,
    pub generated_at: String,
    #[serde(default)]
    pub attempt_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_agent: Option<String>,
    #[serde(default)]
    pub tool_calls_recent: Vec<ToolCallSummary>,
    #[serde(default)]
    pub trace_phases: Vec<TracePhaseSummary>,
    #[serde(default)]
    pub llm_calls_count: i64,
    #[serde(default)]
    pub tool_calls_count: i64,
    #[serde(default)]
    pub budget_events_count: i64,
    #[serde(default)]
    pub budget_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapSummary {
    pub has_data: bool,
    pub projects_count: i64,
    pub sessions_total: i64,
    pub next_steps: Vec<String>,
    /// e.g. `v2_complete` — signals UI/CLI that V1+V2 ship is done.
    pub workbench_phase: String,
    /// Repo-relative path to the primary V3 planning doc (zh).
    pub planning_doc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_registered: Option<Vec<(String, bool)>>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexAssetsResult {
    pub indexed: usize,
    pub missing: usize,
    pub skipped: usize,
    pub total: usize,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingAgentEntry {
    pub agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReadPolicySummary {
    pub summary: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSettings {
    pub config_path: String,
    pub config_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_model: Option<String>,
    pub routing_agents: Vec<RoutingAgentEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_routes: Option<Value>,
    pub auth_mode: String,
    pub host: String,
    pub port: u16,
    pub db_path: String,
    pub sse_events_path: String,
    pub sse_project_events_path: String,
    pub asset_read_policy: AssetReadPolicySummary,
    pub skills_total: i64,
    pub skills_enabled_links: i64,
    #[serde(default)]
    pub asset_read_strict: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPreferences {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    #[serde(default)]
    pub asset_read_strict: bool,
    #[serde(default = "chrono_now")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMetricPoint {
    pub date: String,
    pub sessions_count: i64,
    pub events_count: i64,
    pub gates_failed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTimelineMetrics {
    pub days: u32,
    pub points: Vec<TimelineMetricPoint>,
    pub trust_trend_pct: f64,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageStats {
    pub days: u32,
    pub llm_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageRow {
    pub model: String,
    pub provider: String,
    pub llm_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageDetail {
    pub usage: TokenUsageStats,
    pub by_model: Vec<ModelUsageRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedHoursKpi {
    pub days: u32,
    pub sessions_completed: i64,
    pub automation_hours: f64,
    pub baseline_hours_per_session: f64,
    pub estimated_manual_hours: f64,
    pub estimated_saved_hours: f64,
    pub hourly_rate_usd: f64,
    pub estimated_value_usd: f64,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentNotification {
    pub id: String,
    pub action: String,
    pub title: String,
    pub detail: String,
    pub created_at: String,
    pub project_id: Option<String>,
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPreferencesView {
    pub active: DashboardPreferences,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved: Option<DashboardPreferences>,
    pub restart_command: String,
    pub preferences_path: String,
    pub restart_required: bool,
}
