import { useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import type { SessionWithProject } from "@/api/types";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { ConversationComposer } from "@/components/ConversationComposer";
import { ConversationTranscript } from "@/components/ConversationTranscript";
import { AskUserQuestionInbox } from "@/components/AskUserQuestionInbox";
import { ExecutionProgressBar } from "@/components/ExecutionProgressBar";
import { SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { Icon } from "@/components/Icon";
import { SessionStatusBadges } from "@/components/ui/StatusBadge";
import { formatDuration, formatRelativeTime } from "@/utils/formatTime";
import { useT } from "@/i18n/context";

interface Props {
  sessions: SessionWithProject[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  pendingCounts?: Map<string, number>;
  onPrefetch?: (sessionId: string, isRunning: boolean) => void;
}

type SessionGroup = "today" | "week" | "earlier";

function sessionGroupKey(startedAt: string): SessionGroup {
  const normalized = startedAt.includes("T") ? startedAt : startedAt.replace(" ", "T");
  const d = new Date(normalized);
  if (Number.isNaN(d.getTime())) return "earlier";
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const startOfWeek = new Date(startOfToday);
  startOfWeek.setDate(startOfWeek.getDate() - 7);
  if (d >= startOfToday) return "today";
  if (d >= startOfWeek) return "week";
  return "earlier";
}

function statusDotClass(status: string, trusted: string): string {
  if (trusted === "blocked") return "bg-error";
  if (status === "running") return "bg-primary animate-pulse";
  if (status === "failed") return "bg-error";
  if (status === "completed") return "bg-secondary";
  return "bg-outline";
}

export function ConversationSessionList({
  sessions,
  selectedId,
  onSelect,
  pendingCounts,
  onPrefetch,
}: Props) {
  const t = useT();

  if (sessions.length === 0) {
    return <p className="text-sm text-secondary px-3 py-4 m-0">{t("conversations.noSessions")}</p>;
  }

  const grouped: Record<SessionGroup, SessionWithProject[]> = {
    today: [],
    week: [],
    earlier: [],
  };
  for (const s of sessions) {
    grouped[sessionGroupKey(s.started_at)].push(s);
  }

  const sections: { key: SessionGroup; label: string }[] = [
    { key: "today", label: t("conversations.listGroupToday") },
    { key: "week", label: t("conversations.listGroupWeek") },
    { key: "earlier", label: t("conversations.listGroupEarlier") },
  ];

  return (
    <div className="py-1">
      {sections.map(({ key, label }) => {
        const rows = grouped[key];
        if (rows.length === 0) return null;
        return (
          <section key={key}>
            <h4 className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 sticky top-0 bg-surface-container-lowest/95 backdrop-blur-sm z-[1]">
              {label}
            </h4>
            <ul className="m-0 p-0 list-none">
              {rows.map((s) => {
                const active = s.id === selectedId;
                const pending = pendingCounts?.get(s.id) ?? 0;
                return (
                  <li key={s.id} className="group">
                    <button
                      type="button"
                      onClick={() => onSelect(s.id)}
                      onMouseEnter={() =>
                        onPrefetch?.(s.id, s.status === "running")
                      }
                      onFocus={() => onPrefetch?.(s.id, s.status === "running")}
                      className={`w-full text-left px-3 py-2 border-0 cursor-pointer transition-colors flex items-center gap-2 min-w-0 ${
                        active
                          ? "bg-surface-container-high"
                          : "bg-transparent hover:bg-surface-container-low"
                      }`}
                    >
                      <span
                        className={`shrink-0 w-2 h-2 rounded-full ${statusDotClass(s.status, s.trusted_status)}`}
                        title={s.status}
                      />
                      <span className="min-w-0 flex-1">
                        <span className="text-sm font-medium truncate block">
                          {s.title || s.id}
                        </span>
                        <span className="text-[11px] text-secondary truncate block">
                          {formatRelativeTime(s.started_at)}
                          {pending > 0 && (
                            <span className="text-warn ml-1">
                              · {t("home.securityPendingBadge").replace("{n}", String(pending))}
                            </span>
                          )}
                        </span>
                      </span>
                      <span className="shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                        <SessionStatusBadges
                          status={s.status}
                          trustedStatus={s.trusted_status}
                          pendingApprovalCount={pending}
                        />
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </div>
  );
}

export function ConversationThread({
  session,
  onFollowUpStarted,
  showHeader = true,
  sseLive = false,
}: {
  session: SessionWithProject | null;
  onFollowUpStarted?: (sessionId: string) => void;
  showHeader?: boolean;
  sseLive?: boolean;
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

      <ExecutionProgressBar sessionId={session.id} isRunning={running} sseLive={sseLive} />
      <AskUserQuestionInbox sessionId={session.id} />

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
            sseLive={sseLive}
            scrollContainerRef={scrollRef}
            promptPreview={session.prompt_preview}
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
