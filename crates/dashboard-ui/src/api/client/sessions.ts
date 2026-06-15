import type {
  ArtifactDetail,
  ArtifactRecord,
  GateRecord,
  ProjectEvent,
  ReportDocument,
  SessionDetail,
  SessionFacetsResponse,
  SessionReplaySummary,
  SessionTraceResponse,
  SessionTranscriptResponse,
  ExecutionLogResponse,
  SessionWithProject,
  TokenUsageDetail,
} from "../types";
import { get, post } from "../http";
import { buildArtifactQuery, type ArtifactListOpts, type EventListOpts, type SessionListOpts } from "./shared";

export const sessionsClient = {
  allSessions: (opts?: SessionListOpts) => {
    const q = new URLSearchParams();
    q.set("limit", String(opts?.limit ?? 100));
    if (opts?.kind) q.set("kind", opts.kind);
    if (opts?.status) q.set("status", opts.status);
    if (opts?.trustedStatus) q.set("trusted_status", opts.trustedStatus);
    if (opts?.projectId) q.set("project_id", opts.projectId);
    if (opts?.budgetExceeded) q.set("budget_exceeded", "true");
    return get<{ sessions: SessionWithProject[] }>(`/api/sessions?${q}`);
  },
  sessionFacets: () => get<{ facets: SessionFacetsResponse }>("/api/sessions/facets"),
  sessionsByKind: (kind: string, limit = 50) =>
    get<{ sessions: SessionWithProject[] }>(
      `/api/sessions?limit=${limit}&kind=${encodeURIComponent(kind)}`,
    ),
  session: (sessionId: string) =>
    get<{ session: SessionDetail }>(`/api/sessions/${sessionId}`),
  cancelSession: (sessionId: string) =>
    post<{ ok: boolean; session_id: string; live_signal: boolean }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/cancel`,
      {},
    ),
  acknowledgeSessionBlock: (sessionId: string) =>
    post<{ ok: boolean; session_id: string }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/acknowledge-block`,
      {},
    ),
  sessionAutoApprove: (sessionId: string) =>
    get<{ session_id: string; enabled: boolean }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/auto-approve`,
    ),
  setSessionAutoApprove: (sessionId: string, enabled: boolean) =>
    post<{ ok: boolean; session_id: string; enabled: boolean }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/auto-approve`,
      { enabled },
    ),
  sessionUsage: (sessionId: string) =>
    get<TokenUsageDetail>(
      `/api/sessions/${encodeURIComponent(sessionId)}/usage`,
    ),
  sessionEvents: (sessionId: string, opts?: EventListOpts) => {
    const q = new URLSearchParams({ limit: String(opts?.limit ?? 200) });
    if (opts?.eventType) q.set("event_type", opts.eventType);
    if (opts?.severity) q.set("severity", opts.severity);
    if (opts?.q) q.set("q", opts.q);
    return get<{ events: ProjectEvent[] }>(
      `/api/sessions/${sessionId}/events?${q}`,
    );
  },
  sessionEventTypes: (sessionId: string) =>
    get<{ event_types: string[] }>(`/api/sessions/${sessionId}/event-types`),
  sessionGates: (sessionId: string) =>
    get<{ gates: GateRecord[] }>(`/api/sessions/${sessionId}/gates`),
  artifacts: (opts?: ArtifactListOpts) =>
    get<{ artifacts: ArtifactRecord[] }>(
      `/api/artifacts?${buildArtifactQuery(opts)}`,
    ),
  sessionArtifacts: (sessionId: string, opts?: Omit<ArtifactListOpts, "sessionId">) =>
    get<{ artifacts: ArtifactRecord[] }>(
      `/api/sessions/${sessionId}/artifacts?${buildArtifactQuery({ ...opts, sessionId })}`,
    ),
  scanSessionArtifacts: (sessionId: string) =>
    post<{ ok: boolean; session_id: string; registered: number }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/scan-artifacts`,
      {},
    ),
  sessionReport: (sessionId: string, lang?: string) =>
    get<{ report: ReportDocument }>(
      `/api/sessions/${sessionId}/report${lang ? `?lang=${encodeURIComponent(lang)}` : ""}`,
    ),
  sessionReplay: (sessionId: string) =>
    get<{ replay: SessionReplaySummary }>(`/api/sessions/${sessionId}/replay`),
  sessionTrace: (sessionId: string) =>
    get<{ trace: SessionTraceResponse }>(`/api/sessions/${sessionId}/trace`),
  sessionTranscript: (sessionId: string) =>
    get<{ transcript: SessionTranscriptResponse }>(
      `/api/sessions/${encodeURIComponent(sessionId)}/transcript`,
      { timeoutMs: 45_000 },
    ),
  sessionExecutionLog: (
    sessionId: string,
    opts?: { offset?: number; limit?: number },
  ) => {
    const q = new URLSearchParams();
    if (opts?.offset != null) q.set("offset", String(opts.offset));
    if (opts?.limit != null) q.set("limit", String(opts.limit));
    const qs = q.toString();
    return get<{ execution_log: ExecutionLogResponse }>(
      `/api/sessions/${sessionId}/execution-log${qs ? `?${qs}` : ""}`,
    );
  },
  sessionBackgroundTasks: (sessionId: string) =>
    get<{
      orchestration_tasks: Array<{
        id: string;
        subject?: string;
        status?: string;
        description?: string;
      }>;
      agent_tool_calls: Array<{
        occurred_at: string;
        title: string;
        severity: string;
        tool: string;
        body?: string;
      }>;
    }>(`/api/sessions/${encodeURIComponent(sessionId)}/background-tasks`),
  recentReports: (opts?: {
    projectId?: string;
    sessionId?: string;
    limit?: number;
  }) => {
    const q = new URLSearchParams();
    if (opts?.projectId) q.set("project_id", opts.projectId);
    if (opts?.sessionId) q.set("session_id", opts.sessionId);
    if (opts?.limit) q.set("limit", String(opts.limit));
    const qs = q.toString();
    return get<{ reports: ArtifactRecord[] }>(
      `/api/reports/recent${qs ? `?${qs}` : ""}`,
    );
  },
  artifactDetail: (artifactId: string) =>
    get<{ artifact: ArtifactDetail }>(`/api/artifacts/${artifactId}`),
};
