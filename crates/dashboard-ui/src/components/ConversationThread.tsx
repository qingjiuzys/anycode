import { useRef } from "react";
import { Link } from "@tanstack/react-router";
import type { SessionWithProject } from "@/api/types";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { ConversationTranscript } from "@/components/ConversationTranscript";
import { PendingApprovalBadge, SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { Icon } from "@/components/Icon";
import { SessionStatusBadges } from "@/components/ui/StatusBadge";
import { ConversationCompose } from "@/components/ConversationCompose";
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
                <SessionStatusBadges
                  status={s.status}
                  trustedStatus={s.trusted_status}
                  pendingApprovalCount={pendingCounts?.get(s.id)}
                />
              </div>
              <div className="flex flex-wrap items-center gap-2 mt-1 text-xs text-secondary">
                <span>{s.kind}</span>
                <span>·</span>
                <span title={s.started_at}>{formatRelativeTime(s.started_at)}</span>
                {s.block_reason && (
                  <>
                    <span>·</span>
                    <span className="text-error line-clamp-1" title={s.block_reason}>
                      {s.block_reason}
                    </span>
                  </>
                )}
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
  onFollowUpStarted,
}: {
  session: SessionWithProject | null;
  onFollowUpStarted?: (sessionId: string) => void;
}) {
  const t = useT();
  const scrollRef = useRef<HTMLDivElement>(null);

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
            <div className="mt-1">
              <SessionStatusBadges
                status={session.status}
                trustedStatus={session.trusted_status}
              />
            </div>
            {session.block_reason && (
              <p className="text-xs text-error m-0 mt-1">{session.block_reason}</p>
            )}
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

      <div ref={scrollRef} className="flex-1 overflow-y-auto min-h-0">
        <div className="px-4 py-6 max-w-4xl mx-auto w-full">
          <ConversationTranscript
            sessionId={session.id}
            isRunning={session.status === "running"}
            scrollContainerRef={scrollRef}
          />
        </div>
      </div>

      <div className="px-4 py-3 border-t border-outline-variant bg-surface-container-low shrink-0">
        <ConversationCompose session={session} onSent={onFollowUpStarted} />
      </div>
    </div>
  );
}
