import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { ApprovalDecision, PendingApprovalsResponse } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { sessionChatSearch } from "@/lib/sessionLinks";

type Props = {
  sessionId?: string;
  /** Hide the whole card when there are no pending rows (session detail). */
  hideWhenEmpty?: boolean;
  /** Shorter title on session detail. */
  compact?: boolean;
  /** Inline fold inside the chat transcript (last turn). */
  inline?: boolean;
};

/** Live pending tool approvals — respond from Web when CLI session is recording. */
export function SecurityApprovalInbox({
  sessionId,
  hideWhenEmpty,
  compact,
  inline = false,
}: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const pendingQueryKey = ["security-approvals-pending", sessionId ?? ""] as const;

  const inbox = useQuery({
    queryKey: pendingQueryKey,
    queryFn: () => api.pendingApprovals({ limit: 10, sessionId }),
    staleTime: 5_000,
    refetchInterval: sessionId ? 4_000 : 12_000,
    refetchIntervalInBackground: false,
  });

  const respond = useMutation({
    mutationFn: ({
      approvalId,
      decision,
    }: {
      approvalId: string;
      decision: ApprovalDecision;
    }) => api.respondToApproval(approvalId, decision),
    onMutate: async ({ approvalId }) => {
      await queryClient.cancelQueries({ queryKey: pendingQueryKey });
      const previous = queryClient.getQueryData<PendingApprovalsResponse>(pendingQueryKey);
      if (previous) {
        queryClient.setQueryData<PendingApprovalsResponse>(pendingQueryKey, {
          ...previous,
          pending: previous.pending.filter((row) => row.approval_id !== approvalId),
        });
      }
      return { previous };
    },
    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(pendingQueryKey, context.previous);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["security-approvals-pending"] });
      queryClient.invalidateQueries({ queryKey: ["security-approvals-summary"] });
      queryClient.invalidateQueries({ queryKey: ["security-activity"] });
    },
  });

  const data = inbox.data;
  const rows = data?.pending ?? [];
  const canRespond = data?.respond_allowed ?? false;
  const webEnabled = data?.web_enabled ?? true;
  const title = compact ? t("session.securityInbox") : t("home.securityInbox");

  if (inbox.isLoading) {
    if (hideWhenEmpty) return null;
    return (
      <SectionCard title={title}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </SectionCard>
    );
  }

  if (!webEnabled) {
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
      {rows.map((row) => (
        <ApprovalRow
          key={row.approval_id}
          row={row}
          inline={inline}
          sessionId={sessionId}
          canRespond={canRespond}
          respondPending={respond.isPending}
          onRespond={(approvalId, decision) => respond.mutate({ approvalId, decision })}
          t={t}
        />
      ))}
    </div>
  );

  if (inline) {
    return (
      <div className="chat-trace chat-trace-approval">
        <div className="chat-trace-toggle-static">
          <span className="inline-flex items-center gap-1.5 text-warn">
            <Icon name="policy" size={16} />
            {title}
          </span>
        </div>
        <p className="text-xs text-secondary m-0 mt-1">{hint}</p>
        {!canRespond && (
          <p className="text-xs text-warn m-0 mt-1">{t("home.securityInboxRemoteBlocked")}</p>
        )}
        <div className="mt-2">{rowList}</div>
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
  return (
    <div
      className={
        inline
          ? "rounded-xl border border-warn/30 bg-warn/5 p-3"
          : "dw-card p-3 border border-outline-variant"
      }
    >
      <div className="flex flex-wrap items-start justify-between gap-2 mb-2">
        <div>
          <code className="font-code text-sm">{row.tool}</code>
          <span className="text-xs text-secondary ml-2">{row.created_at}</span>
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
      <pre className="text-xs text-secondary m-0 mb-3 max-h-24 overflow-auto whitespace-pre-wrap font-code bg-surface-container-low p-2 rounded">
        {row.input_preview}
      </pre>
      <div className="flex flex-wrap gap-2">
        <ActionButton
          label={t("home.securityAllowOnce")}
          disabled={!canRespond || respondPending}
          onClick={() => onRespond(row.approval_id, "allow_once")}
        />
        <ActionButton
          label={t("home.securityAllowTool")}
          disabled={!canRespond || respondPending}
          onClick={() => onRespond(row.approval_id, "allow_tool")}
        />
        <ActionButton
          label={t("home.securityAllowAllSession")}
          disabled={!canRespond || respondPending}
          onClick={() => onRespond(row.approval_id, "allow_all_session")}
        />
        <ActionButton
          label={t("home.securityDeny")}
          variant="secondary"
          disabled={!canRespond || respondPending}
          onClick={() => onRespond(row.approval_id, "deny")}
        />
      </div>
    </div>
  );
}

function ActionButton({
  label,
  onClick,
  disabled,
  variant = "primary",
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  variant?: "primary" | "secondary";
}) {
  return (
    <button
      type="button"
      className={variant === "primary" ? "dw-btn-primary text-xs" : "dw-btn-secondary text-xs"}
      disabled={disabled}
      onClick={onClick}
    >
      {label}
    </button>
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
  const bySession = summary.data?.summary.by_session ?? [];
  const counts = new Map(bySession.map((row) => [row.session_id, row.count]));
  return {
    counts,
    pendingTotal: summary.data?.summary.pending_total ?? 0,
    webEnabled: summary.data?.web_enabled ?? true,
    isLoading: summary.isLoading,
  };
}
