import type {
  ApprovalDecision,
  ApprovalSummaryResponse,
  AuditRecord,
  DeliveryReadiness,
  GlobalTimelineMetrics,
  PendingApprovalsResponse,
  SavedHoursKpi,
  SecurityActivitySummary,
  TokenUsageDetail,
  TokenUsageStats,
} from "../types";
import { API_BASE, get, post } from "../http";

export const governanceClient = {
  timelineMetrics: (days = 7) =>
    get<{ timeline: GlobalTimelineMetrics }>(`/api/metrics/timeline?days=${days}`),
  usageMetrics: (days = 7) =>
    get<{ usage: TokenUsageStats; by_model: TokenUsageDetail["by_model"] }>(
      `/api/metrics/usage?days=${days}`,
    ),
  savedHoursKpi: (days = 7) =>
    get<{ kpi: SavedHoursKpi }>(`/api/metrics/kpi/saved-hours?days=${days}`),
  securityActivity: (opts?: { limit?: number; projectId?: string }) => {
    const params = new URLSearchParams();
    if (opts?.limit) params.set("limit", String(opts.limit));
    if (opts?.projectId) params.set("project_id", opts.projectId);
    const q = params.toString();
    return get<{ summary: SecurityActivitySummary }>(
      `/api/security/activity${q ? `?${q}` : ""}`,
    );
  },
  pendingApprovals: (opts?: { limit?: number; sessionId?: string }) => {
    const params = new URLSearchParams();
    if (opts?.limit) params.set("limit", String(opts.limit));
    if (opts?.sessionId) params.set("session_id", opts.sessionId);
    const q = params.toString();
    return get<PendingApprovalsResponse>(
      `/api/security/approvals/pending${q ? `?${q}` : ""}`,
    );
  },
  approvalSummary: () =>
    get<ApprovalSummaryResponse>("/api/security/approvals/summary"),
  respondToApproval: (approvalId: string, decision: ApprovalDecision) =>
    post<{ ok: boolean; approval_id: string; decision: string }>(
      `/api/security/approvals/${encodeURIComponent(approvalId)}/respond`,
      { decision },
    ),
  usageExportUrl: (days = 7, projectId?: string) => {
    const q = new URLSearchParams({ days: String(days) });
    if (projectId) q.set("project_id", projectId);
    return `${API_BASE}/api/metrics/usage/export?${q}`;
  },
  deliveryReadiness: () =>
    get<{ readiness: DeliveryReadiness }>("/api/metrics/readiness"),
  auditEvents: (opts?: {
    projectId?: string;
    action?: string;
    risk?: string;
    limit?: number;
  }) => {
    const q = new URLSearchParams();
    q.set("limit", String(opts?.limit ?? 100));
    if (opts?.projectId) q.set("project_id", opts.projectId);
    if (opts?.action) q.set("action", opts.action);
    if (opts?.risk) q.set("risk", opts.risk);
    return get<{ events: AuditRecord[] }>(`/api/audit/events?${q}`);
  },
};
