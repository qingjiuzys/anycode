import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo, useState, useSyncExternalStore } from "react";
import { api } from "@/api/client";
import type { ApprovalDecision, PendingApprovalsResponse } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import {
  applyApprovalResolvedToCaches,
  invalidateApprovalCaches,
  restoreApprovalResolvedCaches,
  scheduleApprovalSummaryRefresh,
  type ApprovalResolvedCacheSnapshot,
} from "@/lib/approvalCache";
import {
  getOptimisticResolvedApprovalIds,
  markApprovalResolvedOptimistic,
  optimisticResolvedApprovalsEpoch,
  optimisticResolvedApprovalsSnapshot,
  subscribeOptimisticResolvedApprovals,
  unmarkApprovalResolvedOptimistic,
} from "@/lib/approvalOptimisticStore";
import { sessionChatSearch } from "@/lib/sessionLinks";
import type { LivePendingApproval } from "@/lib/sessionLiveStore";

/** Max approval cards shown at once (overflow summarized). */
export const APPROVAL_INBOX_DISPLAY_LIMIT = 5;

type Props = {
  sessionId?: string;
  /** Hide the whole card when there are no pending rows (session detail). */
  hideWhenEmpty?: boolean;
  /** Shorter title on session detail. */
  compact?: boolean;
  /** Inline fold inside the chat transcript (last turn). */
  inline?: boolean;
  /** SSE-driven pending rows (session hot path — no polling). */
  liveApprovals?: LivePendingApproval[];
  /** When using liveApprovals, whether respond is allowed. */
  respondAllowed?: boolean;
};

/** Live pending tool approvals — respond from Web when CLI session is recording. */
export function SecurityApprovalInbox({
  sessionId,
  hideWhenEmpty,
  compact,
  inline = false,
  liveApprovals,
  respondAllowed: respondAllowedProp,
}: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const pendingQueryKey = ["security-approvals-pending", sessionId ?? ""] as const;
  const useLiveFeed = liveApprovals !== undefined;

  const optimisticSnapshot = useSyncExternalStore(
    subscribeOptimisticResolvedApprovals,
    () => optimisticResolvedApprovalsSnapshot(sessionId),
    () => "",
  );
  const optimisticResolved = useMemo(() => {
    void optimisticSnapshot;
    return getOptimisticResolvedApprovalIds(sessionId);
  }, [optimisticSnapshot, sessionId]);

  const liveRows: ApprovalRowData[] = (liveApprovals ?? [])
    .filter((row) => !optimisticResolved.has(row.approval_id))
    .map((row) => ({
      approval_id: row.approval_id,
      session_id: row.session_id,
      tool: row.tool,
      input_preview: row.input_preview,
      created_at: "",
      status: "pending",
    }));

  const inbox = useQuery({
    queryKey: pendingQueryKey,
    queryFn: () => api.pendingApprovals({ limit: 20, sessionId }),
    // Live path relies on sessionLive + rehydrate; do not poll when live feed is wired.
    enabled: !useLiveFeed,
    staleTime: Infinity,
    refetchInterval: !useLiveFeed ? (sessionId ? false : 12_000) : false,
    refetchIntervalInBackground: false,
  });

  const respond = useMutation({
    mutationFn: ({
      approvalId,
      decision,
    }: {
      approvalId: string;
      decision: ApprovalDecision;
      sessionIdForCache?: string;
    }) => api.respondToApproval(approvalId, decision),
    onMutate: async ({ approvalId, sessionIdForCache }) => {
      await queryClient.cancelQueries({ queryKey: ["security-approvals-summary"] });
      await queryClient.cancelQueries({ queryKey: ["pending-approvals-rehydrate"] });
      await queryClient.cancelQueries({ queryKey: ["security-approvals-pending"] });

      const sid =
        sessionIdForCache ||
        sessionId ||
        liveApprovals?.find((row) => row.approval_id === approvalId)?.session_id ||
        inbox.data?.pending.find((row) => row.approval_id === approvalId)?.session_id;

      markApprovalResolvedOptimistic(sid || sessionId, approvalId);

      const cacheSnapshot = applyApprovalResolvedToCaches(queryClient, {
        approvalId,
        sessionId: sid,
      });
      return {
        cacheSnapshot,
        sid: sid || sessionId,
      } satisfies {
        cacheSnapshot: ApprovalResolvedCacheSnapshot;
        sid: string | undefined;
      };
    },
    onError: (_err, { approvalId }, context) => {
      unmarkApprovalResolvedOptimistic(context?.sid || sessionId, approvalId);
      if (context?.cacheSnapshot) {
        restoreApprovalResolvedCaches(queryClient, context.cacheSnapshot);
      }
    },
    onSettled: () => {
      invalidateApprovalCaches(queryClient);
      scheduleApprovalSummaryRefresh(queryClient);
    },
  });

  const data = inbox.data;
  const polledRows: ApprovalRowData[] = (data?.pending ?? []).filter(
    (row) => !optimisticResolved.has(row.approval_id),
  );
  const rows = useLiveFeed ? liveRows : polledRows;
  const visibleRows = rows.slice(0, APPROVAL_INBOX_DISPLAY_LIMIT);
  const overflowCount = Math.max(0, rows.length - visibleRows.length);
  const canRespond = respondAllowedProp ?? data?.respond_allowed ?? true;
  const webEnabled = data?.web_enabled ?? true;
  const title = compact ? t("session.securityInbox") : t("home.securityInbox");

  if (!useLiveFeed && inbox.isLoading && rows.length === 0) {
    if (hideWhenEmpty) return null;
    return (
      <SectionCard title={title}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </SectionCard>
    );
  }

  if (!webEnabled && !useLiveFeed) {
    if (hideWhenEmpty) return null;
    return (
      <SectionCard title={title}>
        <p className="text-sm text-secondary m-0">{t("home.securityInboxDisabled")}</p>
      </SectionCard>
    );
  }

  if (rows.length === 0) {
    if (hideWhenEmpty) return null;
    return (
      <SectionCard title={title}>
        <EmptyState
          title={t("home.securityInboxEmpty")}
          description={t("home.securityInboxHint")}
          icon="policy"
        />
      </SectionCard>
    );
  }

  const hint = sessionId ? t("session.securityInboxHint") : t("home.securityInboxHint");
  const rowList = (
    <div className={inline ? "space-y-2" : "space-y-3"}>
      {visibleRows.map((row) => (
        <ApprovalRow
          key={row.approval_id}
          row={row}
          inline={inline}
          sessionId={sessionId}
          canRespond={canRespond}
          respondPending={respond.isPending}
          onRespond={(approvalId, decision) =>
            respond.mutate({
              approvalId,
              decision,
              sessionIdForCache: row.session_id || sessionId,
            })
          }
          t={t}
        />
      ))}
      {overflowCount > 0 ? (
        <p className="text-xs text-secondary m-0 px-1" data-testid="approval-inbox-overflow">
          {t("home.securityInboxOverflow").replace("{n}", String(overflowCount))}
        </p>
      ) : null}
    </div>
  );

  if (inline) {
    return (
      <div className="approval-inbox approval-inbox--inline" data-testid="approval-inbox">
        {!canRespond && (
          <p className="text-xs text-warn m-0 mb-2">{t("home.securityInboxRemoteBlocked")}</p>
        )}
        {rowList}
      </div>
    );
  }

  return (
    <SectionCard title={title}>
      <p className="text-xs text-secondary m-0 mb-3">{hint}</p>
      {!canRespond && (
        <p className="text-xs text-warn m-0 mb-3">{t("home.securityInboxRemoteBlocked")}</p>
      )}
      {rowList}
    </SectionCard>
  );
}

type ApprovalRowData = PendingApprovalsResponse["pending"][number];

function ApprovalRow({
  row,
  inline,
  sessionId,
  canRespond,
  respondPending,
  onRespond,
  t,
}: {
  row: ApprovalRowData;
  inline?: boolean;
  sessionId?: string;
  canRespond: boolean;
  respondPending: boolean;
  onRespond: (approvalId: string, decision: ApprovalDecision) => void;
  t: ReturnType<typeof useT>;
}) {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const busy = !canRespond || respondPending;
  const previewOneLine = row.input_preview?.trim().replace(/\s+/g, " ") ?? "";
  const previewSnippet =
    previewOneLine.length > 72 ? `${previewOneLine.slice(0, 72)}…` : previewOneLine;

  return (
    <div
      className={
        inline
          ? "approval-card approval-card--inline"
          : "approval-card approval-card--panel"
      }
      data-testid="approval-card"
    >
      <p className="approval-card__summary m-0">
        {t("home.securityNeedsApproval").replace("{tool}", row.tool || "Tool")}
      </p>
      {previewSnippet ? (
        <p className="approval-card__snippet m-0" title={previewOneLine}>
          {previewSnippet}
        </p>
      ) : null}

      <button
        type="button"
        className="approval-card__details-toggle"
        aria-expanded={detailsOpen}
        onClick={() => setDetailsOpen((open) => !open)}
      >
        <span>{t("home.securityToolDetails")}</span>
        <Icon name={detailsOpen ? "expand_more" : "chevron_right"} size={14} />
      </button>

      {detailsOpen && (
        <div className="approval-card__details">
          <div className="flex flex-wrap items-start justify-between gap-2">
            <div>
              <code className="font-code text-sm">{row.tool}</code>
              {row.created_at ? (
                <span className="text-xs text-secondary ml-2">{row.created_at}</span>
              ) : null}
            </div>
            {!sessionId && (
              <Link
                to="/conversations"
                search={sessionChatSearch(row.session_id)}
                className="text-xs text-primary hover:underline"
              >
                {row.session_id}
              </Link>
            )}
          </div>
          {row.input_preview?.trim() ? (
            <pre className="approval-card__preview">{row.input_preview}</pre>
          ) : null}
          <button
            type="button"
            className="approval-card__advanced"
            disabled={busy}
            onClick={() => onRespond(row.approval_id, "allow_all_session")}
          >
            {t("home.securityAllowAllSession")}
          </button>
        </div>
      )}

      <div className="approval-card__actions">
        <button
          type="button"
          className="approval-btn approval-btn--allow"
          disabled={busy}
          onClick={() => onRespond(row.approval_id, "allow_once")}
        >
          {t("home.securityAllowOnce")}
        </button>
        <button
          type="button"
          className="approval-btn approval-btn--allow"
          disabled={busy}
          onClick={() => onRespond(row.approval_id, "allow_tool")}
        >
          {t("home.securityAllowTool")}
        </button>
        <button
          type="button"
          className="approval-btn approval-btn--deny"
          disabled={busy}
          onClick={() => onRespond(row.approval_id, "deny")}
        >
          {t("home.securityDeny")}
        </button>
      </div>
    </div>
  );
}

/** Badge for running session rows when approvals are pending. */
export function PendingApprovalBadge({
  sessionId,
  count,
}: {
  sessionId: string;
  /** When provided, skips the summary query (use on list pages). */
  count?: number;
}) {
  const t = useT();
  const summary = useQuery({
    queryKey: ["security-approvals-summary"],
    queryFn: api.approvalSummary,
    refetchInterval: 12_000,
    enabled: count === undefined,
  });
  const resolved =
    count ??
    summary.data?.summary.by_session.find((row) => row.session_id === sessionId)?.count ??
    0;
  if (resolved === 0) return null;
  return (
    <span className="inline-flex items-center rounded-full bg-warn/15 text-warn text-xs px-2 py-0.5 ml-1">
      {t("home.securityPendingBadge").replace("{n}", String(resolved))}
    </span>
  );
}

/** Map session_id → pending approval count from cached summary query. */
export function usePendingApprovalCounts() {
  const summary = useQuery({
    queryKey: ["security-approvals-summary"],
    queryFn: api.approvalSummary,
    staleTime: 8_000,
    refetchInterval: 12_000,
    refetchIntervalInBackground: false,
  });
  const optimisticEpoch = useSyncExternalStore(
    subscribeOptimisticResolvedApprovals,
    optimisticResolvedApprovalsEpoch,
    () => 0,
  );
  void optimisticEpoch;
  const bySession = summary.data?.summary.by_session ?? [];
  const counts = new Map<string, number>();
  for (const row of bySession) {
    const optimistic = getOptimisticResolvedApprovalIds(row.session_id).size;
    const next = Math.max(0, row.count - optimistic);
    if (next > 0) {
      counts.set(row.session_id, next);
    }
  }
  const pendingTotal = [...counts.values()].reduce((a, b) => a + b, 0);
  return {
    counts,
    pendingTotal,
    webEnabled: summary.data?.web_enabled ?? true,
    isLoading: summary.isLoading,
  };
}
