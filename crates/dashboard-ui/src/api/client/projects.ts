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
  ProjectViewPrefs,
  ProjectsListResponse,
  ReportDocument,
  SessionDetail,
  SessionSummary,
  SkillRecord,
  TokenUsageDetail,
  TokenUsageStats,
} from "../types";
import { del, get, patch, post, put } from "../http";
import type { EventListOpts, ProjectsListOpts } from "./shared";

export interface WebChatResult {
  session_id: string;
  pid: number;
  log_path: string;
  started_at: string;
  queued: boolean;
}

/** Current UI language (same persistence as the i18n provider) for reply-language hints. */
function currentUiLang(): "zh" | "en" {
  const saved = localStorage.getItem("anycode-dashboard-locale");
  if (saved === "en" || saved === "zh") return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export const projectsClient = {
  projects: (opts?: ProjectsListOpts) => {
    const q = new URLSearchParams();
    q.set("limit", String(opts?.limit ?? 100));
    q.set("offset", String(opts?.offset ?? 0));
    if (opts?.q) q.set("q", opts.q);
    if (opts?.status) q.set("status", opts.status);
    if (opts?.sort) q.set("sort", opts.sort);
    return get<ProjectsListResponse>(`/api/projects?${q}`);
  },
  projectTemplates: () =>
    get<{
      templates: Array<{
        id: string;
        name: string;
        name_zh?: string;
        description: string;
        description_zh?: string;
        default_dir: string;
      }>;
    }>("/api/project-templates"),
  upsertProject: (body: {
    root_path: string;
    name?: string;
    description?: string;
    create_root?: boolean;
    template_id?: string;
    app_title?: string;
    bundle_org?: string;
  }) =>
    post<{ project: ProjectDetail }>("/api/projects", body),
  scanProjects: () =>
    post<{
      ok: boolean;
      projects_registered: number;
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
  startConversation: (
    projectId: string,
    body: {
      title?: string;
      prompt: string;
      kind?: "run" | "goal";
      goal?: string;
      agent?: string;
      skills?: string[];
      vision_images?: { mime_type: string; data_base64: string }[];
      text_files?: { filename: string; content: string }[];
      lang?: string;
    },
  ) =>
    post<{ session: SessionDetail; chat: WebChatResult }>(
      `/api/projects/${encodeURIComponent(projectId)}/conversations/start`,
      { lang: currentUiLang(), ...body },
    ),
  sendSessionMessage: (
    sessionId: string,
    body: {
      prompt: string;
      agent?: string;
      skills?: string[];
      vision_images?: { mime_type: string; data_base64: string }[];
      text_files?: { filename: string; content: string }[];
      lang?: string;
    },
  ) =>
    post<{ ok: boolean; session_id: string; chat: WebChatResult }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/message`,
      { lang: currentUiLang(), ...body },
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
  projectKnowledgePaths: (projectId: string) =>
    get<{ paths: string[] }>(`/api/projects/${projectId}/knowledge`),
  setProjectKnowledgePaths: (projectId: string, paths: string[]) =>
    put<{ ok: boolean; paths: string[] }>(`/api/projects/${projectId}/knowledge`, {
      paths,
    }),
  reindexProjectKnowledge: (projectId: string) =>
    post<{ ok: boolean; chunks_indexed: number }>(
      `/api/projects/${projectId}/knowledge/reindex`,
    ),
  projectKnowledgeStats: (projectId: string) =>
    get<{
      stats: {
        path_count: number;
        chunk_count: number;
        cache_path?: string | null;
        cache_bytes?: number | null;
        vectors_enabled?: boolean;
        vector_count?: number;
        vector_store_path?: string | null;
      };
    }>(`/api/projects/${projectId}/knowledge/stats`),
  searchProjectKnowledge: (projectId: string, q: string, limit = 8) =>
    get<{
      hits: Array<{ source_file: string; snippet: string; score: number }>;
    }>(
      `/api/projects/${projectId}/knowledge/search?q=${encodeURIComponent(q)}&limit=${limit}`,
    ),
  projectReport: (projectId: string, lang?: string) =>
    get<{ report: ReportDocument }>(
      `/api/projects/${projectId}/report${lang ? `?lang=${encodeURIComponent(lang)}` : ""}`,
    ),
  patchProjectStatus: (projectId: string, status: string) =>
    patch<{ ok: boolean; project_id: string; status: string }>(
      `/api/projects/${encodeURIComponent(projectId)}/status`,
      { status },
    ),
  renameProject: (projectId: string, name: string) =>
    patch<{ ok: boolean; project_id: string; name: string }>(
      `/api/projects/${encodeURIComponent(projectId)}`,
      { name },
    ),
  projectViewPrefs: (projectId: string) =>
    get<{ view_prefs: ProjectViewPrefs }>(
      `/api/projects/${encodeURIComponent(projectId)}/view-prefs`,
    ),
  setProjectViewPrefs: (projectId: string, prefs: ProjectViewPrefs) =>
    put<{ ok: boolean; view_prefs: ProjectViewPrefs }>(
      `/api/projects/${encodeURIComponent(projectId)}/view-prefs`,
      prefs,
    ),
};
