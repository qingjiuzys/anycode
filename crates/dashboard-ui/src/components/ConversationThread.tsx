import { useMemo } from "react";
import { Link } from "@tanstack/react-router";
import type { ProjectEvent, SessionWithProject } from "@/api/types";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { PendingApprovalBadge, SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { Icon } from "@/components/Icon";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { formatDuration, formatRelativeTime } from "@/utils/formatTime";
import { useT } from "@/i18n/context";

interface Props {
  sessions: SessionWithProject[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  pendingCounts?: Map<string, number>;
}

export function ConversationSessionList({
  sessions,
  selectedId,
  onSelect,
  pendingCounts,
}: Props) {
  const t = useT();

  if (sessions.length === 0) {
    return <p className="text-sm text-secondary px-3 py-4 m-0">{t("conversations.noSessions")}</p>;
  }

  return (
    <ul className="m-0 p-0 list-none divide-y divide-outline-variant">
      {sessions.map((s) => {
        const active = s.id === selectedId;
        return (
          <li key={s.id}>
            <button
              type="button"
              onClick={() => onSelect(s.id)}
              className={`w-full text-left px-3 py-2.5 border-0 cursor-pointer transition-colors ${
                active
                  ? "bg-surface-container-high"
                  : "bg-transparent hover:bg-surface-container-low"
              }`}
            >
              <div className="flex items-start justify-between gap-2">
                <span className="text-sm font-medium line-clamp-2">
                  {s.title}
                  <PendingApprovalBadge
                    sessionId={s.id}
                    count={pendingCounts?.get(s.id)}
                  />
                </span>
                <StatusBadge status={s.status} />
              </div>
              <div className="flex flex-wrap items-center gap-2 mt-1 text-xs text-secondary">
                <span>{s.kind}</span>
                <span>·</span>
                <StatusBadge status={s.trusted_status} />
                <span>·</span>
                <span title={s.started_at}>{formatRelativeTime(s.started_at)}</span>
              </div>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

export function ConversationThread({
  session,
  events,
  loading,
}: {
  session: SessionWithProject | null;
  events: ProjectEvent[];
  loading: boolean;
}) {
  const t = useT();

  const messages = useMemo(() => groupEvents(events), [events]);

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-secondary p-8">
        <Icon name="forum" size={40} className="opacity-40 mb-3" />
        <p className="m-0 text-sm">{t("conversations.selectSession")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full min-h-[420px]">
      <div className="px-4 py-3 border-b border-outline-variant bg-surface-container-low shrink-0">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <h3 className="text-base font-semibold m-0">{session.title}</h3>
            <p className="text-xs text-secondary m-0 mt-0.5">
              {session.kind} · {formatDuration(session.started_at, session.ended_at)} ·{" "}
              {session.agent_type || "—"}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <CancelSessionButton sessionId={session.id} status={session.status} compact />
            <Link
              to="/sessions/$sessionId"
              params={{ sessionId: session.id }}
              className="dw-btn-secondary text-xs no-underline"
            >
              {t("conversations.openDetail")}
            </Link>
          </div>
        </div>
      </div>

      {session.status === "running" && (
        <div className="px-4 py-3 border-b border-outline-variant bg-surface-container-lowest shrink-0">
          <SecurityApprovalInbox sessionId={session.id} hideWhenEmpty compact />
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {loading && <p className="text-sm text-secondary">{t("common.loading")}</p>}
        {!loading && messages.length === 0 && (
          <p className="text-sm text-secondary">{t("conversations.noMessages")}</p>
        )}
        {messages.map((m) => (
          <div
            key={m.id}
            className={`max-w-[92%] rounded-lg px-3 py-2 text-sm ${
              m.role === "user"
                ? "ml-auto bg-primary text-on-primary"
                : m.role === "error"
                  ? "bg-error-container text-on-error-container border border-error/20"
                  : "bg-surface-container-low border border-outline-variant"
            }`}
          >
            <div className="text-[10px] uppercase font-semibold opacity-70 mb-1">{m.label}</div>
            <div className="whitespace-pre-wrap break-words">{m.body}</div>
            <div className="text-[10px] opacity-60 mt-1 flex items-center gap-2">
              <time>{formatRelativeTime(m.at)}</time>
              {m.eventId && (
                <Link
                  to="/events/$eventId"
                  params={{ eventId: m.eventId }}
                  className={m.role === "user" ? "text-on-primary underline" : "text-primary"}
                >
                  {t("common.details")}
                </Link>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="px-4 py-3 border-t border-outline-variant bg-surface-container-low shrink-0">
        <p className="text-xs text-secondary m-0">{t("conversations.readOnlyHint")}</p>
      </div>
    </div>
  );
}

interface ThreadMessage {
  id: string;
  role: "user" | "assistant" | "tool" | "error" | "system";
  label: string;
  body: string;
  at: string;
  eventId?: string;
}

function groupEvents(events: ProjectEvent[]): ThreadMessage[] {
  const conversationTypes = new Set([
    "user_prompt",
    "prompt",
    "assistant_response",
    "tool_call_start",
    "tool_call_end",
    "tool_approval_pending",
    "tool_approval_resolved",
    "tool_denied",
    "task_start",
    "task_end",
  ]);
  const sorted = [...events]
    .filter(
      (e) =>
        conversationTypes.has(e.event_type) ||
        e.event_type.includes("user") ||
        e.event_type.includes("assistant") ||
        e.severity === "error",
    )
    .sort((a, b) => a.occurred_at.localeCompare(b.occurred_at));
  return sorted.map((e) => {
    let role: ThreadMessage["role"] = "system";
    if (e.event_type.includes("user") || e.event_type === "prompt" || e.event_type === "user_prompt") {
      role = "user";
    } else if (e.event_type.startsWith("tool_call")) role = "tool";
    else if (e.severity === "error" || e.event_type === "tool_denied") role = "error";
    else if (
      e.event_type === "tool_approval_pending" ||
      e.event_type === "tool_approval_resolved"
    ) {
      role = "system";
    }
    else if (
      e.event_type.includes("assistant") ||
      e.event_type === "assistant_response"
    ) {
      role = "assistant";
    }

    const body = (e.body || e.title).trim();
    return {
      id: e.id,
      role,
      label: e.event_type,
      body: body.length > 2000 ? `${body.slice(0, 2000)}…` : body,
      at: e.occurred_at,
      eventId: e.id,
    };
  });
}
