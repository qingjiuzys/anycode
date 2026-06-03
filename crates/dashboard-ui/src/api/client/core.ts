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
import { get, post } from "../http";
import type { AuthUser } from "./shared";

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
    schedule_timezone?: string;
    session_id?: string;
    failure_destination?: string;
    tool_profile?: string;
  }) => post<{ ok: boolean; job: CronJobRecord }>("/api/cron/jobs", body),
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
  agentStats: (limit = 30) =>
    get<{ agents: AgentUsageStat[] }>(`/api/agents/stats?limit=${limit}`),
};
