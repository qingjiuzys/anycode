import type {
  ApiTokenRecord,
  ConnectorRecord,
  DashboardPreferencesView,
  DataHealth,
  DbOperations,
  DoctorReport,
  GithubIssueSummary,
  LinearIssueSummary,
  LocalServiceRecord,
  NotificationPolicyRecord,
  PolicySummary,
  RecentNotification,
  RuntimeSettings,
  ServiceStatusDetail,
  SkillDetailRecord,
  SkillRecord,
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
    model_fallback_provider?: string | null;
    model_fallback_model?: string | null;
  }) =>
    put<{ ok: boolean; preferences: DashboardPreferencesView }>(
      "/api/settings/preferences",
      body,
    ),
  patchLlmConfig: (body: {
    provider?: string;
    model?: string;
    fallback_provider?: string;
    fallback_model?: string;
  }) =>
    put<{
      ok: boolean;
      config_path: string;
      llm?: unknown;
      model_fallback?: unknown;
    }>("/api/settings/llm", body),
  policies: () => get<{ policy: PolicySummary }>("/api/settings/policies"),
  dataHealth: () => get<{ health: DataHealth }>("/api/settings/data-health"),
  serviceStatus: () =>
    get<{ service: ServiceStatusDetail }>("/api/settings/service-status"),
  dbOperations: () =>
    get<{ operations: DbOperations }>("/api/settings/db-operations"),
  apiTokens: () => get<{ tokens: ApiTokenRecord[] }>("/api/settings/tokens"),
  createToken: (name: string, expiresDays?: number) =>
    post<{ token: ApiTokenRecord; plaintext: string }>(
      "/api/settings/tokens",
      { name, expires_days: expiresDays },
    ),
  revokeToken: (tokenId: string) =>
    post<{ ok: boolean }>(`/api/settings/tokens/${tokenId}/revoke`),
  skills: (limit = 80) => get<{ skills: SkillRecord[] }>(`/api/skills?limit=${limit}`),
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
