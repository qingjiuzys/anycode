import type { QueryClient } from "@tanstack/react-query";
import type { ApprovalSummaryResponse, PendingApprovalsResponse } from "@/api/types";

export type ApprovalResolvedCacheSnapshot = {
  summary: ApprovalSummaryResponse | undefined;
  rehydrate: Array<[readonly unknown[], PendingApprovalsResponse | undefined]>;
  pending: Array<[readonly unknown[], PendingApprovalsResponse | undefined]>;
};

/** Pure: drop one approval from a pending list response. */
export function removeApprovalFromPending(
  data: PendingApprovalsResponse | undefined,
  approvalId: string,
): PendingApprovalsResponse | undefined {
  if (!data) return data;
  const next = data.pending.filter((row) => row.approval_id !== approvalId);
  if (next.length === data.pending.length) return data;
  return { ...data, pending: next };
}

/** Pure: decrement session count / pending_total after one approval resolves. */
export function removeApprovalFromSummary(
  data: ApprovalSummaryResponse | undefined,
  sessionId: string | undefined,
): ApprovalSummaryResponse | undefined {
  if (!data) return data;
  const by_session = data.summary.by_session
    .map((row) => {
      if (!sessionId || row.session_id !== sessionId) return row;
      return { ...row, count: Math.max(0, row.count - 1) };
    })
    .filter((row) => row.count > 0);
  const pending_total = Math.max(0, data.summary.pending_total - 1);
  return {
    ...data,
    summary: { pending_total, by_session },
  };
}

/**
 * Optimistically clear one pending approval from React Query caches that drive
 * the sidebar badge and live rehydrate merge.
 */
export function applyApprovalResolvedToCaches(
  queryClient: QueryClient,
  opts: { approvalId: string; sessionId?: string },
): ApprovalResolvedCacheSnapshot {
  const { approvalId, sessionId } = opts;
  const summaryKey = ["security-approvals-summary"] as const;
  const previousSummary = queryClient.getQueryData<ApprovalSummaryResponse>(summaryKey);
  const nextSummary = removeApprovalFromSummary(previousSummary, sessionId);
  if (nextSummary !== previousSummary) {
    queryClient.setQueryData(summaryKey, nextSummary);
  }

  const rehydrateEntries = queryClient.getQueriesData<PendingApprovalsResponse>({
    queryKey: ["pending-approvals-rehydrate"],
  });
  const previousRehydrate: ApprovalResolvedCacheSnapshot["rehydrate"] = [];
  for (const [key, value] of rehydrateEntries) {
    previousRehydrate.push([key, value]);
    const next = removeApprovalFromPending(value, approvalId);
    if (next !== value) {
      queryClient.setQueryData(key, next);
    }
  }

  const pendingEntries = queryClient.getQueriesData<PendingApprovalsResponse>({
    queryKey: ["security-approvals-pending"],
  });
  const previousPending: ApprovalResolvedCacheSnapshot["pending"] = [];
  for (const [key, value] of pendingEntries) {
    previousPending.push([key, value]);
    const next = removeApprovalFromPending(value, approvalId);
    if (next !== value) {
      queryClient.setQueryData(key, next);
    }
  }

  return {
    summary: previousSummary,
    rehydrate: previousRehydrate,
    pending: previousPending,
  };
}

export function restoreApprovalResolvedCaches(
  queryClient: QueryClient,
  snapshot: ApprovalResolvedCacheSnapshot,
): void {
  queryClient.setQueryData(["security-approvals-summary"], snapshot.summary);
  for (const [key, value] of snapshot.rehydrate) {
    queryClient.setQueryData(key, value);
  }
  for (const [key, value] of snapshot.pending) {
    queryClient.setQueryData(key, value);
  }
}

export function invalidateApprovalCaches(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: ["security-approvals-pending"] });
  void queryClient.invalidateQueries({ queryKey: ["pending-approvals-rehydrate"] });
  void queryClient.invalidateQueries({ queryKey: ["security-activity"] });
}

/** Soft refresh summary after IPC TTL so we don't immediately re-poison optimistic clears. */
export function scheduleApprovalSummaryRefresh(
  queryClient: QueryClient,
  delayMs = 2_600,
): void {
  window.setTimeout(() => {
    void queryClient.invalidateQueries({ queryKey: ["security-approvals-summary"] });
  }, delayMs);
}
