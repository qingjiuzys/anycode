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

export interface MaskedSecret {
  configured: boolean;
  preview?: string | null;
}

export interface ModelFallbackView {
  provider?: string | null;
  model?: string | null;
  on?: "geo" | "rate_limit" | "any_error";
}

export interface ModelProfile {
  provider?: string | null;
  model?: string | null;
  plan?: string | null;
  base_url?: string | null;
  api_key?: string | null;
  temperature?: number | null;
  max_tokens?: number | null;
}

export interface SpeechModelsConfig {
  stt?: ModelProfile | null;
  tts?: ModelProfile | null;
}

export interface ModelsConfig {
  chat?: ModelProfile | null;
  embedding?: ModelProfile | null;
  speech?: SpeechModelsConfig | null;
  image?: ModelProfile | null;
  video?: ModelProfile | null;
}

export interface LlmConfigView {
  config_present: boolean;
  provider?: string | null;
  model?: string | null;
  plan?: string | null;
  base_url?: string | null;
  api_key: MaskedSecret;
  provider_credentials: Record<string, MaskedSecret>;
  model_fallback: ModelFallbackView;
  models: ModelsConfig;
  routing_agents?: Record<string, ModelProfile> | null;
  registry?: {
    active: Record<string, string>;
    items: Array<{
      id: string;
      display_name?: string | null;
      provider: string;
      model: string;
      capabilities: string[];
      enabled: boolean;
      source?: string | null;
    }>;
  };
}

export interface CatalogProviderRow {
  id: string;
  label: string;
  hint?: string | null;
  transport: string;
  suggested_openai_base?: string | null;
  placeholder_only: boolean;
}

export interface CatalogModelRow {
  id: string;
  label: string;
  description?: string | null;
  capabilities?: string[];
}

export interface CatalogAuthMethod {
  label: string;
  hint?: string | null;
  plan: string;
}

export interface CatalogRoutingPreset {
  id: string;
  hint: string;
}

export interface LocalMediaPreset {
  id: string;
  label: string;
  description: string;
  capabilities: string[];
  mode: "builtin" | "external" | "platform_native";
  provider: string;
  model: string;
  base_url?: string | null;
  voice?: string | null;
  docs_url?: string | null;
  model_download_hint?: string | null;
  required_feature?: string | null;
  feature_available: boolean;
  desktop_only?: boolean;
}

export interface LocalPresetsView {
  presets: LocalMediaPreset[];
  lightweight_bundle: string[];
  build_features: {
    embedding_local: boolean;
    stt_local: boolean;
    tts_local: boolean;
    media_local: boolean;
  };
}

export interface ModelCatalog {
  providers: CatalogProviderRow[];
  zai_models: CatalogModelRow[];
  google_models: CatalogModelRow[];
  /** DeepSeek V4 + legacy aliases (official API docs). */
  deepseek_models?: CatalogModelRow[];
  provider_models?: Record<string, CatalogModelRow[]>;
  zai_auth_methods: CatalogAuthMethod[];
  routing_agent_presets: CatalogRoutingPreset[];
  capabilities: { id: string; label?: string }[];
  cache_meta?: Record<string, CatalogRefreshMeta>;
  local_presets?: LocalPresetsView;
}

export interface CatalogRefreshMeta {
  last_refreshed_at?: string | null;
  source: string;
  offline_cache_used: boolean;
  refresh_error?: string | null;
}

export interface ConfiguredModel {
  id: string;
  display_name?: string | null;
  provider: string;
  model: string;
  capabilities: string[];
  plan?: string | null;
  base_url?: string | null;
  api_key?: string | null;
  api_key_ref?: string | null;
  temperature?: number | null;
  max_tokens?: number | null;
  extra_headers?: Record<string, string> | null;
  endpoint_overrides?: EndpointOverrides | null;
  enabled: boolean;
  tags?: string[] | null;
  source?: string | null;
}

export interface EndpointOverrides {
  submit?: string | null;
  status?: string | null;
  result?: string | null;
}

export interface ModelsRegistryView {
  config_present: boolean;
  active: Record<string, string>;
  items: ConfiguredModel[];
  routing?: Record<string, unknown> | null;
  model_fallback: ModelFallbackView;
  global?: { provider?: string | null; model?: string | null };
}

export interface PutModelsBody {
  items?: ConfiguredModel[];
  active?: Record<string, string>;
  delete_ids?: string[];
}

export interface RefreshCatalogResult {
  ok: boolean;
  provider: string;
  models: CatalogModelRow[];
  meta: CatalogRefreshMeta;
  error?: string;
}

export interface LlmConfigPatchBody {
  provider?: string;
  model?: string;
  plan?: string;
  base_url?: string;
  api_key?: string;
  provider_credentials?: Record<string, string>;
  fallback_provider?: string;
  fallback_model?: string;
  fallback_on?: string;
  routing_agents?: Record<string, ModelProfile>;
  routing_agents_delete?: string[];
  models?: ModelsConfig;
}

export interface LlmConfigPatchResult {
  ok: boolean;
  config_path: string;
  provider?: string | null;
  model?: string | null;
  model_fallback?: ModelFallbackView | null;
  models?: ModelsConfig | null;
}

export interface TestLlmResult {
  ok: boolean;
  message?: string;
  error?: string;
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

export type ReportOutputFormat = "markdown" | "html" | "both";
export type ReportGenerationMode = "llm" | "template";

export interface DashboardPreferences {
  host: string;
  port: number;
  db_path: string;
  asset_read_strict?: boolean;
  report_output_format?: ReportOutputFormat;
  report_generation_mode?: ReportGenerationMode;
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
