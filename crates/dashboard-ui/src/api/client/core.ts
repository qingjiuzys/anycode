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
  agentStats: (limit = 30) =>
    get<{ agents: AgentUsageStat[] }>(`/api/agents/stats?limit=${limit}`),
};
