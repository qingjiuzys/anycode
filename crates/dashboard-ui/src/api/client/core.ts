import type {
  AgentUsageStat,
  BootstrapSummary,
  CronJobRecord,
  CronRunRecord,
  HealthResponse,
  OverviewStats,
  ProjectEvent,
  RecentEvent,
  SearchResults,
  SessionWithProject,
  ToolGovernanceResponse,
} from "../types";
import { get, post, del, patch } from "../http";
import type { AuthUser } from "./shared";

export interface LocalModelStatus {
  id: string;
  version: string;
  display_name: string;
  model_id: string;
  file_name: string;
  download_url: string;
  sha256: string;
  size_bytes: number;
  architectures: string[];
  context_tokens: number;
  minimum_ram_bytes: number;
  license: string;
  capabilities: { chat: boolean; tools: boolean; vision: boolean };
  runtime: string;
  runtime_args: string[];
  preview: boolean;
  phase: string;
  model_path?: string | null;
  base_url?: string | null;
  port?: number | null;
  download_bytes: number;
  download_total: number;
  disk_free_bytes?: number | null;
  ram_total_bytes?: number | null;
  last_error?: string | null;
}

export const coreClient = {
  health: () => get<HealthResponse>("/api/health"),
  authMe: async (): Promise<{ authenticated: boolean; user?: AuthUser }> => {
    try {
      return await get<{ authenticated: boolean; user?: AuthUser }>("/api/auth/me");
    } catch (err) {
      if (err instanceof Error && err.message.startsWith("401 ")) {
        return { authenticated: false };
      }
      throw err;
    }
  },
  login: (email: string, password: string) =>
    post<{ authenticated: boolean; user: AuthUser }>("/api/auth/login", {
      email,
      password,
    }),
  logout: () => post<{ ok: boolean }>("/api/auth/logout"),
  bootstrap: () => get<{ bootstrap: BootstrapSummary }>("/api/bootstrap"),
  overview: () => get<{ overview: OverviewStats }>("/api/overview"),
  overviewBriefing: (days = 7, lang: "zh" | "en" = "zh") =>
    post<{ briefing: import("../types").OverviewBriefing }>(
      `/api/overview/briefing?days=${days}&lang=${lang}`,
      {},
      { timeoutMs: 90_000 },
    ),
  toolGovernance: () => get<ToolGovernanceResponse>("/api/governance/tools"),
  recentEvents: () => get<{ events: RecentEvent[] }>("/api/events?limit=40"),
  event: (eventId: string) => get<{ event: ProjectEvent }>(`/api/events/${eventId}`),
  runningSessions: () =>
    get<{ sessions: SessionWithProject[] }>("/api/sessions/running?limit=20"),
  search: (q: string, limit = 15) =>
    get<SearchResults>(`/api/search?q=${encodeURIComponent(q)}&limit=${limit}`),
  cronRuns: (limit = 30) =>
    get<{ runs: CronRunRecord[]; ledger_path?: string }>(
      `/api/cron/runs?limit=${limit}`,
    ),
  cronJobs: () =>
    get<{ jobs: CronJobRecord[]; orchestration_path?: string }>("/api/cron/jobs"),
  deleteCronJob: (jobId: string) =>
    del<{ ok: boolean; job_id: string }>(`/api/cron/jobs/${encodeURIComponent(jobId)}`),
  retryCronJob: (body: { job_id: string; project_id?: string }) =>
    post<{ ok: boolean; job_id: string; trigger: unknown }>("/api/cron/retry", body),
  skillSuggestions: () =>
    get<{
      missing_starter: string[];
      usage: Array<{ skill_id: string; count: number }>;
      installed_count: number;
    }>("/api/skills/suggestions"),
  createCronJob: (body: {
    schedule: string;
    command: string;
    name?: string;
    enabled?: boolean;
    schedule_timezone?: string;
    session_id?: string;
    failure_destination?: string;
    tool_profile?: string;
    project_id?: string;
  }) => post<{ ok: boolean; job: CronJobRecord; schedule_note?: string }>("/api/cron/jobs", body),
  patchCronJob: (
    jobId: string,
    body: {
      name?: string;
      enabled?: boolean;
      schedule?: string;
      command?: string;
      schedule_timezone?: string;
      session_id?: string;
      failure_destination?: string;
      tool_profile?: string;
      project_id?: string;
    },
  ) =>
    patch<{ ok: boolean; job: CronJobRecord; next_run_at?: string }>(
      `/api/cron/jobs/${encodeURIComponent(jobId)}`,
      body,
    ),
  parseCronSchedule: (text: string) =>
    post<{ ok: boolean; schedule: string; summary: string }>(
      "/api/cron/parse-schedule",
      { text },
    ),
  installStarterSkills: () =>
    post<{ ok: boolean; installed: string[]; count: number }>(
      "/api/skills/install-starter",
      {},
    ),
  cronTemplates: () =>
    get<{ templates: Record<string, unknown>[] }>("/api/cron/templates"),
  orchestrationTasks: () =>
    get<{ tasks: Record<string, unknown>; teams: Record<string, unknown> }>(
      "/api/orchestration/tasks",
    ),
  importSkill: (source: string) =>
    post<{ ok: boolean; id: string; path: string }>("/api/skills/import", { source }),
  installMarketSkill: (id: string) =>
    post<{ ok: boolean; id: string; path: string }>(
      "/api/skills/market/install",
      { id },
      { timeoutMs: 180_000 },
    ),
  skillMarket: () =>
    get<{
      market: {
        entries: import("@/api/types").SkillMarketEntry[];
      };
    }>("/api/skills/market"),
  agentStats: (limit = 30) =>
    get<{ agents: AgentUsageStat[] }>(`/api/agents/stats?limit=${limit}`),
  cloudSession: () =>
    get<{
      linked: boolean;
      identity_verified: boolean;
      portal_url?: string | null;
      gateway_url?: string | null;
      user_email?: string | null;
      display_name?: string | null;
      access_token?: string | null;
    }>("/api/cloud/session"),
  cloudLinkStart: () =>
    post<{
      device_code: string;
      user_code?: string | null;
      verification_uri: string;
      verification_uri_complete?: string | null;
      expires_in?: number | null;
      browser_url: string;
      redirect_uri: string;
    }>("/api/cloud/link/start", {}),
  cloudLinkPoll: (device_code: string) =>
    post<{ linked: boolean; pending?: boolean; error?: string }>("/api/cloud/link/poll", {
      device_code,
    }),
  cloudGatewayTest: () =>
    post<{
      ok: boolean;
      status?: number;
      gateway?: string;
      snippet?: string;
      error?: string;
    }>("/api/cloud/gateway-test", {}),
  cloudSyncModels: () =>
    post<{ ok: boolean; synced: number; error?: string }>("/api/cloud/sync-models", {}),
  cloudUnlink: () => post<{ ok: boolean; removed?: number }>("/api/cloud/unlink", {}),
  localModels: () => get<{ models: LocalModelStatus[] }>("/api/local-models"),
  localModelDownload: (id: string) =>
    post<{ ok: boolean; error?: string }>(
      `/api/local-models/${encodeURIComponent(id)}/download`,
      {},
    ),
  localModelCancelDownload: (id: string) =>
    post<{ ok: boolean }>(
      `/api/local-models/${encodeURIComponent(id)}/download/cancel`,
      {},
    ),
  localModelStart: (id: string) =>
    post<{ ok: boolean; error?: string }>(
      `/api/local-models/${encodeURIComponent(id)}/start`,
      {},
    ),
  localModelStop: (id: string) =>
    post<{ ok: boolean; error?: string }>(
      `/api/local-models/${encodeURIComponent(id)}/stop`,
      {},
    ),
  localModelDelete: (id: string) =>
    del<{ ok: boolean; error?: string }>(`/api/local-models/${encodeURIComponent(id)}`),
  managedLocalStatus: () =>
    get<{
      id: string;
      phase: string;
      model_path?: string | null;
      base_url?: string | null;
      port?: number | null;
      download_bytes: number;
      download_total: number;
      context_tokens: number;
      tool_calls_supported: boolean;
      last_error?: string | null;
    }>("/api/local-llm/status"),
  managedLocalDownload: () => post<{ ok: boolean; error?: string }>("/api/local-llm/download", {}),
  managedLocalCancelDownload: () =>
    post<{ ok: boolean }>("/api/local-llm/download/cancel", {}),
  managedLocalStart: () => post<{ ok: boolean; error?: string }>("/api/local-llm/start", {}),
  managedLocalStop: () => post<{ ok: boolean; error?: string }>("/api/local-llm/stop", {}),
  managedLocalDelete: () => del<{ ok: boolean; error?: string }>("/api/local-llm/model"),
};
