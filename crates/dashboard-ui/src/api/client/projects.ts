import type {
  AutomationPolicyRecord,
  DataHealth,
  GateExecuteResult,
  GatePreset,
  GateRecord,
  IndexAssetsResult,
  ProjectDetail,
  ProjectEvent,
  ProjectMetrics,
  ProjectStats,
  ProjectSummary,
  ReportDocument,
  SessionSummary,
  SkillRecord,
  TokenUsageDetail,
  TokenUsageStats,
  TriggerRunResult,
} from "../types";
import { del, get, post, put } from "../http";
import type { EventListOpts } from "./shared";

export const projectsClient = {
  projects: () => get<{ projects: ProjectSummary[] }>("/api/projects"),
  upsertProject: (body: { root_path: string; name?: string; description?: string }) =>
    post<{ project: ProjectDetail }>("/api/projects", body),
  scanProjects: () =>
    post<{
      ok: boolean;
      projects_registered: number;
      ingested_tasks: number;
      skills_synced: number;
    }>("/api/projects/scan"),
  project: (id: string) => get<{ project: ProjectDetail }>(`/api/projects/${id}`),
  projectStats: (projectId: string) =>
    get<{ stats: ProjectStats }>(`/api/projects/${projectId}/stats`),
  sessions: (projectId: string) =>
    get<{ sessions: SessionSummary[] }>(
      `/api/projects/${projectId}/sessions?limit=50`,
    ),
  projectEventTypes: (projectId: string) =>
    get<{ event_types: string[] }>(`/api/projects/${projectId}/event-types`),
  events: (projectId: string, opts?: EventListOpts) => {
    const q = new URLSearchParams({ limit: String(opts?.limit ?? 100) });
    if (opts?.eventType) q.set("event_type", opts.eventType);
    if (opts?.severity) q.set("severity", opts.severity);
    if (opts?.q) q.set("q", opts.q);
    return get<{ events: ProjectEvent[] }>(
      `/api/projects/${projectId}/events?${q}`,
    );
  },
  reindexProject: (projectId: string) =>
    post<{
      ok: boolean;
      ingested_tasks: number;
      skills_synced: number;
    }>(`/api/projects/${projectId}/reindex`),
  gates: (projectId: string) =>
    get<{ gates: GateRecord[] }>(`/api/projects/${projectId}/gates`),
  projectUsage: (projectId: string, days = 7) =>
    get<{ usage: TokenUsageStats; by_model: TokenUsageDetail["by_model"] }>(
      `/api/projects/${encodeURIComponent(projectId)}/usage?days=${days}`,
    ),
  gatePresets: (projectId: string) =>
    get<{ presets: GatePreset[] }>(
      `/api/projects/${encodeURIComponent(projectId)}/gates/presets`,
    ),
  executeGate: (
    projectId: string,
    body: { preset_id?: string; name?: string; command?: string; required?: boolean },
  ) =>
    post<{ result: GateExecuteResult }>(
      `/api/projects/${encodeURIComponent(projectId)}/gates/execute`,
      body,
    ),
  triggerProjectRun: (
    projectId: string,
    body: {
      prompt: string;
      kind?: "run" | "goal";
      goal?: string;
      agent?: string;
    },
  ) =>
    post<{ trigger: TriggerRunResult }>(
      `/api/projects/${encodeURIComponent(projectId)}/runs/trigger`,
      body,
    ),
  listProjectTriggers: (projectId: string, limit = 10) =>
    get<{ triggers: TriggerRunResult[] }>(
      `/api/projects/${encodeURIComponent(projectId)}/runs/triggers?limit=${limit}`,
    ),
  indexProjectAssets: (projectId: string) =>
    post<{ ok: boolean; result: IndexAssetsResult }>(
      `/api/projects/${projectId}/index-assets`,
    ),
  projectDataHealth: (projectId: string) =>
    get<{ health: DataHealth }>(`/api/projects/${projectId}/data-health`),
  projectMetrics: (projectId: string) =>
    get<{ metrics: ProjectMetrics }>(`/api/projects/${projectId}/metrics`),
  automationPolicies: (projectId: string) =>
    get<{ policies: AutomationPolicyRecord[] }>(
      `/api/projects/${projectId}/automation-policies`,
    ),
  upsertAutomationPolicy: (
    projectId: string,
    body: {
      name: string;
      policy_type: string;
      config: Record<string, unknown>;
      enabled?: boolean;
      id?: string;
    },
  ) =>
    post<{ policy: AutomationPolicyRecord }>(
      `/api/projects/${projectId}/automation-policies`,
      body,
    ),
  deleteAutomationPolicy: (projectId: string, policyId: string) =>
    del<{ ok: boolean }>(
      `/api/projects/${projectId}/automation-policies/${policyId}`,
    ),
  setProjectSkill: (projectId: string, skillId: string, enabled: boolean) =>
    put<{ ok: boolean }>(`/api/projects/${projectId}/skills/${skillId}`, {
      enabled,
    }),
  projectSkills: (projectId: string) =>
    get<{ skills: SkillRecord[] }>(`/api/projects/${projectId}/skills`),
  projectReport: (projectId: string) =>
    get<{ report: ReportDocument }>(`/api/projects/${projectId}/report`),
};
