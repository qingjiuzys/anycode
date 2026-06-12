import type {
  ApiTokenRecord,
  ConfiguredModel,
  ConnectorRecord,
  DashboardPreferencesView,
  DataHealth,
  DbOperations,
  DoctorReport,
  GithubIssueSummary,
  LinearIssueSummary,
  LlmConfigPatchBody,
  LlmConfigPatchResult,
  LlmConfigView,
  LocalServiceRecord,
  ModelCatalog,
  ModelsRegistryView,
  NotificationPolicyRecord,
  PolicySummary,
  PutModelsBody,
  RecentNotification,
  RefreshCatalogResult,
  RuntimeSettings,
  ServiceStatusDetail,
  SkillDetailRecord,
  SkillRecord,
  TestLlmResult,
} from "../types";
import { del, get, patch, post, put } from "../http";

export const settingsClient = {
  services: () => get<{ services: LocalServiceRecord[] }>("/api/settings/services"),
  database: () => get<{ path: string; driver: string }>("/api/settings/database"),
  doctor: () => get<{ doctor: DoctorReport }>("/api/settings/doctor"),
  runtimeSettings: () => get<{ runtime: RuntimeSettings }>("/api/settings/runtime"),
  dashboardPreferences: () =>
    get<{ preferences: DashboardPreferencesView }>("/api/settings/preferences"),
  saveDashboardPreferences: (body: {
    host: string;
    port: number;
    db_path: string;
    asset_read_strict?: boolean;
    report_output_format?: string;
    report_generation_mode?: string;
    model_fallback_provider?: string | null;
    model_fallback_model?: string | null;
  }) =>
    put<{ ok: boolean; preferences: DashboardPreferencesView }>(
      "/api/settings/preferences",
      body,
    ),
  modelCatalog: () => get<ModelCatalog>("/api/settings/model-catalog"),
  refreshModelCatalog: (body?: { provider?: string; base_url?: string }) =>
    post<RefreshCatalogResult>("/api/settings/model-catalog/refresh", body ?? {}),
  getModelsRegistry: () => get<ModelsRegistryView>("/api/settings/models"),
  putModelsRegistry: (body: PutModelsBody) =>
    put<{ ok: boolean; config_path: string }>("/api/settings/models", body),
  enableModel: (modelId: string, capabilities: string[]) =>
    post<{ ok: boolean }>(`/api/settings/models/${encodeURIComponent(modelId)}/enable`, {
      capabilities,
    }),
  testModel: (
    modelId: string,
    body?: { capability?: string; draft?: ConfiguredModel },
  ) =>
    post<TestLlmResult>(
      `/api/settings/models/${encodeURIComponent(modelId)}/test`,
      body ?? {},
    ),
  getLlmConfig: () => get<LlmConfigView>("/api/settings/llm"),
  putLlmConfig: (body: LlmConfigPatchBody) =>
    put<LlmConfigPatchResult>("/api/settings/llm", body),
  testLlm: (capability: string) =>
    post<TestLlmResult>("/api/settings/llm", { capability }),
  policies: () => get<{ policy: PolicySummary }>("/api/settings/policies"),
  dataHealth: () => get<{ health: DataHealth }>("/api/settings/data-health"),
  serviceStatus: () =>
    get<{ service: ServiceStatusDetail }>("/api/settings/service-status"),
  dbOperations: () =>
    get<{ operations: DbOperations }>("/api/settings/db-operations"),
  memoryRetentionPreview: (olderThanDays = 90) =>
    get<{
      rows: unknown[];
      summary: { would_delete: number; keep: number; protected: number };
      older_than_days: number;
    }>(`/api/settings/memory/retention?older_than_days=${olderThanDays}`),
  memoryRetentionApply: (olderThanDays: number, confirm: boolean) =>
    post<{
      rows: unknown[];
      summary: { would_delete: number; keep: number; protected: number };
      older_than_days: number;
    }>("/api/settings/memory/retention", {
      older_than_days: olderThanDays,
      confirm,
    }),
  apiTokens: () => get<{ tokens: ApiTokenRecord[] }>("/api/settings/tokens"),
  createToken: (name: string, expiresDays?: number) =>
    post<{ token: ApiTokenRecord; plaintext: string }>(
      "/api/settings/tokens",
      { name, expires_days: expiresDays },
    ),
  revokeToken: (tokenId: string) =>
    post<{ ok: boolean }>(`/api/settings/tokens/${tokenId}/revoke`),
  skills: (limit = 80) =>
    get<{ skills: SkillRecord[]; scan_roots?: number }>(`/api/skills?limit=${limit}`),
  rescanSkills: () => post<{ ok: boolean; skills_synced: number }>("/api/skills"),
  skillDetail: (skillId: string) =>
    get<{ skill: SkillDetailRecord }>(`/api/skills/${skillId}`),
  setSkillAllProjects: (skillId: string, enabled: boolean) =>
    post<{ ok: boolean; projects_updated: number }>(
      `/api/skills/${skillId}/all-projects`,
      { enabled },
    ),
  notificationPolicies: (projectId?: string) => {
    const q = projectId ? `?project_id=${encodeURIComponent(projectId)}` : "";
    return get<{ policies: NotificationPolicyRecord[] }>(
      `/api/settings/notifications${q}`,
    );
  },
  upsertNotificationPolicy: (body: {
    event_type: string;
    channel: string;
    config?: Record<string, unknown>;
    enabled?: boolean;
    project_id?: string;
    id?: string;
  }) =>
    post<{ policy: NotificationPolicyRecord }>(
      "/api/settings/notifications",
      body,
    ),
  testNotification: (eventType: string, projectId?: string) =>
    post<{ ok: boolean }>("/api/settings/notifications/test", {
      event_type: eventType,
      project_id: projectId,
    }),
  deleteNotificationPolicy: (policyId: string) =>
    del<{ ok: boolean }>(`/api/settings/notifications/${policyId}`),
  setNotificationPolicyEnabled: (policyId: string, enabled: boolean) =>
    patch<{ policy: NotificationPolicyRecord }>(
      `/api/settings/notifications/${policyId}/enabled`,
      { enabled },
    ),
  browserConnector: () =>
    get<{
      enabled: boolean;
      bundled: boolean;
      chromium_ready: boolean;
      bundle_path?: string;
    }>("/api/settings/browser-connector"),
  setBrowserConnector: (enabled: boolean) =>
    put<{ ok: boolean; enabled: boolean; restart_hint?: string }>(
      "/api/settings/browser-connector",
      { enabled },
    ),
  mcpServers: () =>
    get<{ servers: Record<string, unknown>[] }>("/api/settings/mcp-servers"),
  setMcpServers: (servers: unknown[]) =>
    put<{ ok: boolean; servers: unknown[]; restart_hint?: string }>(
      "/api/settings/mcp-servers",
      { servers },
    ),
  promptPreview: (params?: { agent?: string; cwd?: string }) => {
    const q = new URLSearchParams();
    if (params?.agent) q.set("agent", params.agent);
    if (params?.cwd) q.set("cwd", params.cwd);
    const suffix = q.toString() ? `?${q.toString()}` : "";
    return get<{
      agent: string;
      cwd: string;
      system_prompt_override?: string | null;
      system_prompt_append?: string | null;
      segments: { id: string; text: string; chars: number }[];
      composed: string;
    }>(`/api/settings/prompt-preview${suffix}`);
  },
  setPromptSettings: (body: {
    system_prompt_append?: string | null;
    system_prompt_override?: string | null;
  }) =>
    put<{
      ok: boolean;
      config_path?: string;
      restart_hint?: string;
    }>("/api/settings/prompt-settings", body),
  connectors: (projectId?: string) => {
    const q = projectId ? `?project_id=${encodeURIComponent(projectId)}` : "";
    return get<{ connectors: ConnectorRecord[] }>(
      `/api/settings/connectors${q}`,
    );
  },
  upsertConnector: (body: {
    source_type: string;
    name: string;
    config?: Record<string, unknown>;
    enabled?: boolean;
    project_id?: string;
    id?: string;
  }) => post<{ connector: ConnectorRecord }>("/api/settings/connectors", body),
  deleteConnector: (connectorId: string) =>
    del<{ ok: boolean }>(`/api/settings/connectors/${connectorId}`),
  setConnectorEnabled: (connectorId: string, enabled: boolean) =>
    patch<{ connector: ConnectorRecord }>(
      `/api/settings/connectors/${connectorId}/enabled`,
      { enabled },
    ),
  githubIssues: (connectorId: string) =>
    get<{ issues: GithubIssueSummary[]; repo: string }>(
      `/api/settings/connectors/${encodeURIComponent(connectorId)}/github/issues`,
    ),
  linearIssues: (connectorId: string) =>
    get<{ issues: LinearIssueSummary[]; team: string }>(
      `/api/settings/connectors/${encodeURIComponent(connectorId)}/linear/issues`,
    ),
  notificationsRecent: (limit = 20) =>
    get<{ notifications: RecentNotification[] }>(
      `/api/notifications/recent?limit=${limit}`,
    ),
};
