import type { LabelCount } from "./core";

export interface ProjectDetail {
  id: string;
  name: string;
  root_path: string;
  description: string;
  business_goal: string;
  status: string;
  trust_score?: number | null;
  automation_level: number;
  created_at: string;
  updated_at: string;
}

export interface ProjectSummary {
  id: string;
  name: string;
  root_path: string;
  status: string;
  trust_score?: number | null;
  sessions_count: number;
  artifacts_count: number;
  updated_at: string;
  root_exists?: boolean;
}

export interface ProjectsListResponse {
  projects: ProjectSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface ProjectStatsFailure {
  id: string;
  title: string;
  event_type: string;
  occurred_at: string;
  session_id?: string | null;
}

export interface ProjectStats {
  event_types: LabelCount[];
  severities: LabelCount[];
  session_statuses: LabelCount[];
  gate_statuses: LabelCount[];
  recent_failures: ProjectStatsFailure[];
}

export interface ProjectEvent {
  id: string;
  project_id: string;
  session_id: string | null;
  task_id?: string | null;
  agent_id?: string | null;
  event_type: string;
  severity: string;
  title: string;
  body: string;
  payload?: Record<string, unknown>;
  occurred_at: string;
}

export interface ProjectReadinessItem {
  project_id: string;
  project_name: string;
  readiness_score: number;
  blocked_sessions: number;
  failed_gates: number;
  unverified_artifacts: number;
}

export interface ProjectMetrics {
  project_id: string;
  sessions_total: number;
  sessions_completed: number;
  blocked_sessions: number;
  failed_required_gates: number;
  unverified_artifacts: number;
  stale_running_sessions: number;
  events_7d: number;
  gate_pass_rate: number;
  session_success_rate: number;
  readiness_score: number;
  generated_at: string;
}

export interface TriggerRunResult {
  trigger_id: string;
  project_id: string;
  kind: string;
  pid: number;
  command_preview: string;
  log_path: string;
  started_at: string;
  sandbox_note?: string;
}

export interface ProjectViewPrefs {
  sessionFlowLimit: number;
  hideImportedSessions: boolean;
  acceptancePresetIds: string[];
}
