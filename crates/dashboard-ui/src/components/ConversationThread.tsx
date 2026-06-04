import { useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import type { SessionWithProject } from "@/api/types";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { ConversationComposer } from "@/components/ConversationComposer";
import { ConversationTranscript } from "@/components/ConversationTranscript";
import { PendingApprovalBadge, SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { Icon } from "@/components/Icon";
import { SessionStatusBadges } from "@/components/ui/StatusBadge";
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
  showHeader = true,
}: {
  session: SessionWithProject | null;
  onFollowUpStarted?: (sessionId: string) => void;
  showHeader?: boolean;
}) {
  const t = useT();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [metaOpen, setMetaOpen] = useState(false);

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-secondary p-8">
        <Icon name="forum" size={40} className="opacity-40 mb-3" />
        <p className="m-0 text-sm">{t("conversations.selectSession")}</p>
      </div>
    );
  }

  const running = session.status === "running";

  return (
    <div className="flex flex-col h-full min-h-0">
      {showHeader && (
        <div className="px-4 py-2.5 border-b border-outline-variant bg-surface-container-low shrink-0">
          <div className="flex items-center justify-between gap-2">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 min-w-0">
                <h3 className="text-base font-semibold m-0 truncate">{session.title}</h3>
                <SessionStatusBadges
                  status={session.status}
                  trustedStatus={session.trusted_status}
                />
              </div>
              {session.block_reason && (
                <p className="text-xs text-error m-0 mt-1 truncate" title={session.block_reason}>
                  {session.block_reason}
                </p>
              )}
            </div>
            <div className="flex items-center gap-1 shrink-0">
              <div className="relative">
                <button
                  type="button"
                  className="dw-btn-ghost p-1.5"
                  aria-expanded={metaOpen}
                  aria-label={t("common.details")}
                  onClick={() => setMetaOpen((v) => !v)}
                >
                  <Icon name="more_horiz" size={18} />
                </button>
                {metaOpen && (
                  <>
                    <button
                      type="button"
                      className="fixed inset-0 z-10 cursor-default"
                      aria-hidden
                      onClick={() => setMetaOpen(false)}
                    />
                    <div className="absolute right-0 top-full mt-1 z-20 min-w-[14rem] rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg p-3 text-xs text-secondary">
                      <p className="m-0">
                        {session.kind} · {formatDuration(session.started_at, session.ended_at)}
                      </p>
                      <p className="m-0 mt-1">
                        {session.agent_type || "—"} · {session.model || "—"}
                      </p>
                      <Link
                        to="/sessions/$sessionId"
                        params={{ sessionId: session.id }}
                        className="inline-flex items-center gap-1 mt-2 text-primary no-underline hover:underline"
                        onClick={() => setMetaOpen(false)}
                      >
                        <Icon name="open_in_new" size={14} />
                        {t("conversations.openDetail")}
                      </Link>
                    </div>
                  </>
                )}
              </div>
              <CancelSessionButton sessionId={session.id} status={session.status} compact />
            </div>
          </div>
        </div>
      )}

      {running && (
        <div className="px-4 py-2 border-b border-outline-variant bg-surface-container-lowest shrink-0">
          <SecurityApprovalInbox sessionId={session.id} hideWhenEmpty compact />
        </div>
      )}

      <div ref={scrollRef} className="flex-1 overflow-y-auto min-h-0">
        <div className="px-4 py-6 max-w-3xl mx-auto w-full">
          <ConversationTranscript
            sessionId={session.id}
            isRunning={running}
            scrollContainerRef={scrollRef}
          />
        </div>
      </div>

      <div className="shrink-0 border-t border-outline-variant bg-surface-container-low">
        <ConversationComposer
          mode="follow-up"
          session={session}
          onSent={onFollowUpStarted}
        />
      </div>
    </div>
  );
}
