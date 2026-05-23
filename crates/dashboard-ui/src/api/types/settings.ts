export interface LocalServiceRecord {
  name: string;
  host: string;
  port: number;
  status: string;
  auth_mode: string;
}

export interface PolicySummary {
  mode: string;
  host_binding: string;
  remote_access_allowed: boolean;
  write_actions_allowed: boolean;
  safe_actions: string[];
  blocked_actions: string[];
}

export interface HealthCheckItem {
  id: string;
  name: string;
  status: string;
  message: string;
  count: number;
  project_id?: string | null;
  session_id?: string | null;
}

export interface DataHealth {
  status: string;
  db_path: string;
  db_size_bytes: number;
  generated_at: string;
  checks: HealthCheckItem[];
}

export interface ServiceStatusDetail {
  name: string;
  host: string;
  port: number;
  status: string;
  auth_mode: string;
  version: string;
  pid?: number | null;
  started_at: string;
  db_path: string;
  ui_dist?: string | null;
  ui_dist_present: boolean;
  sse_subscribers: number;
  last_event_at?: string | null;
  loopback: boolean;
}

export interface DoctorReport {
  status: string;
  generated_at: string;
  checks: DoctorCheck[];
  next_steps?: string[];
}

export interface DoctorCheck {
  id: string;
  status: string;
  message: string;
}

export interface RoutingAgentEntry {
  agent: string;
  provider?: string | null;
  model?: string | null;
}

export interface AssetReadPolicySummary {
  summary: string;
  rules: string[];
}

export interface RuntimeSettings {
  config_path: string;
  config_present: boolean;
  global_provider?: string | null;
  global_model?: string | null;
  routing_agents: RoutingAgentEntry[];
  model_routes?: Record<string, unknown> | null;
  auth_mode: string;
  host: string;
  port: number;
  db_path: string;
  sse_events_path: string;
  sse_project_events_path: string;
  asset_read_policy: AssetReadPolicySummary;
  skills_total: number;
  skills_enabled_links: number;
  asset_read_strict?: boolean;
  fallback_provider?: string | null;
  fallback_model?: string | null;
}

export interface DashboardPreferences {
  host: string;
  port: number;
  db_path: string;
  asset_read_strict?: boolean;
  model_fallback_provider?: string | null;
  model_fallback_model?: string | null;
  updated_at: string;
}

export interface DashboardPreferencesView {
  active: DashboardPreferences;
  saved?: DashboardPreferences | null;
  restart_command: string;
  preferences_path: string;
  restart_required: boolean;
}

export interface NotificationPolicyRecord {
  id: string;
  project_id?: string | null;
  event_type: string;
  channel: string;
  enabled: boolean;
  config: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ConnectorRecord {
  id: string;
  project_id?: string | null;
  source_type: string;
  name: string;
  enabled: boolean;
  config_summary: string;
  created_at: string;
  updated_at: string;
}

export interface DbOperations {
  db_path: string;
  db_size_bytes: number;
  migrations: { version: number; name: string; applied_at: string }[];
  tables: { name: string; row_count: number }[];
  backup_suggestion: string;
  growth_warnings: string[];
  health_status: string;
  generated_at: string;
}

export interface ApiTokenRecord {
  id: string;
  name: string;
  prefix: string;
  created_at: string;
  expires_at?: string | null;
  last_used_at?: string | null;
  revoked: boolean;
}
