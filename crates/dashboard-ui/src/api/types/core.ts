export interface HealthResponse {
  ok: boolean;
  version: string;
  db_path: string;
  mode: string;
  account_api_url?: string | null;
}

export interface OverviewStats {
  projects_count: number;
  sessions_total: number;
  sessions_running: number;
  sessions_blocked: number;
  sessions_budget_exceeded: number;
  artifacts_count: number;
  skills_count: number;
  gates_failed: number;
  events_last_hour: number;
}

export interface BootstrapSummary {
  has_data: boolean;
  projects_count: number;
  sessions_total: number;
  next_steps: string[];
  workbench_phase: string;
  planning_doc: string;
  workspace_registered?: [string, boolean][];
  generated_at: string;
}

export interface RecentEvent {
  id: string;
  project_id: string;
  project_name: string;
  session_id: string | null;
  event_type: string;
  severity: string;
  title: string;
  occurred_at: string;
}

export interface LabelCount {
  label: string;
  count: number;
}

export interface CronRunRecord {
  job_id: string;
  session_id: string;
  fired_at: string;
  status: string;
  detail: string;
  line_no: number;
  dashboard_session_id: string | null;
}

export interface CronJobRecord {
  id: string;
  schedule: string;
  command: string;
  session_id: string | null;
  failure_destination: string | null;
  tool_profile: string | null;
  project_id?: string | null;
}

export interface AgentUsageStat {
  agent_type: string;
  model: string;
  sessions_count: number;
  last_started_at: string | null;
}

export interface SearchHit {
  kind: string;
  id: string;
  title: string;
  subtitle: string;
  href?: string | null;
  project_id?: string | null;
  session_id?: string | null;
}

export interface SearchResults {
  query: string;
  projects: SearchHit[];
  sessions: SearchHit[];
  events: SearchHit[];
}

export interface ReportSummary {
  sessions: number;
  events: number;
  failed_gates: number;
  artifacts: number;
}

export interface ReportSourceCounts {
  sessions: number;
  events: number;
  gates: number;
  artifacts: number;
}

export interface ReportHighlights {
  trust_verified: number;
  trust_unverified: number;
  trust_blocked: number;
  failures_unique: number;
  verdict: string;
}

export interface ReportSessionRow {
  session_id: string;
  title: string;
  kind: string;
  status: string;
  trusted_status: string;
  started_at: string;
  is_imported?: boolean;
}

export interface ReportFailureGroup {
  title: string;
  event_type: string;
  count: number;
  last_at: string;
  session_id?: string | null;
}

export interface ReportGateRow {
  name: string;
  status: string;
  required: boolean;
  output_excerpt: string;
}

export interface ReportArtifactRow {
  path: string;
  kind: string;
  trust_level: string;
}

export interface ReportDocument {
  scope: string;
  id: string;
  title: string;
  format: string;
  generated_at: string;
  trusted_status: string;
  markdown: string;
  html?: string | null;
  generation_mode?: string;
  summary: ReportSummary;
  source_counts: ReportSourceCounts;
  lang?: string;
  highlights?: ReportHighlights;
  sessions_recent?: ReportSessionRow[];
  sessions_imported_count?: number;
  failure_groups?: ReportFailureGroup[];
  gates?: ReportGateRow[];
  artifacts?: ReportArtifactRow[];
  project_id?: string | null;
  root_path?: string | null;
  events_sample_limit?: number;
}

export interface RecentNotification {
  id: string;
  action: string;
  title: string;
  detail: string;
  created_at: string;
  project_id?: string | null;
}
