import { useRef, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { SessionWithProject } from "@/api/types";
import { ConversationComposer } from "@/components/ConversationComposer";
import { ConversationTranscript } from "@/components/ConversationTranscript";
import { Icon } from "@/components/Icon";
import { SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { AskUserQuestionInbox } from "@/components/AskUserQuestionInbox";
import { SessionTitleMenu } from "@/components/session/SessionTitleMenu";
import { ConversationGitBar } from "@/components/session/ConversationGitBar";
import { SessionStatusBadges, SessionRunningDots } from "@/components/ui/StatusBadge";
import { formatRelativeTime } from "@/utils/formatTime";
import { useT } from "@/i18n/context";
import { findActiveToolInExecutionLog } from "@/lib/transcriptGrouping";
import { hasTurnStreamActivity } from "@/lib/liveTranscript";
import { hideComposerWaitingFromSession } from "@/lib/turnLiveStatus";
import type { SessionLiveState } from "@/lib/sessionLiveStore";
import type { SseStatus } from "@/hooks/useEventSource";
import { useConversationShell } from "@/context/ConversationShellContext";
import {
  conversationStreamLive,
  conversationThreadRunning,
} from "@/hooks/useSessionEventStream";

interface Props {
  sessions: SessionWithProject[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  pendingCounts?: Map<string, number>;
  onPrefetch?: (sessionId: string, isRunning: boolean) => void;
  optimisticStreamingSessionId?: string | null;
}

type SessionGroup = "today" | "week" | "earlier";

function sessionRunningVisual(
  session: SessionWithProject,
  optimisticStreamingSessionId?: string | null,
): boolean {
  return session.status === "running" || optimisticStreamingSessionId === session.id;
}

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

function statusDotClass(status: string, trusted: string, runningVisual: boolean): string {
  if (trusted === "blocked") return "bg-error";
  if (runningVisual) return "bg-primary animate-pulse";
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
  optimisticStreamingSessionId = null,
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
            <h4 className="px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-secondary m-0 sticky top-0 bg-surface-container-lowest/95 backdrop-blur-sm z-[1]">
              {label}
            </h4>
            <ul className="m-0 p-0 list-none">
              {rows.map((s) => {
                const active = s.id === selectedId;
                const pending = pendingCounts?.get(s.id) ?? 0;
                const runningVisual = sessionRunningVisual(s, optimisticStreamingSessionId);
                const showAlertBadge =
                  s.status === "failed" || s.trusted_status === "blocked" || pending > 0;
                return (
                  <li key={s.id} className="group">
                    <button
                      type="button"
                      onClick={() => onSelect(s.id)}
                      onMouseEnter={() =>
                        onPrefetch?.(s.id, s.status === "running")
                      }
                      onFocus={() => onPrefetch?.(s.id, s.status === "running")}
                      className={`w-full text-left px-3 py-2 border-0 cursor-pointer transition-colors flex items-start gap-2 min-w-0 ${
                        active
                          ? "bg-surface-container-high"
                          : "bg-transparent hover:bg-surface-container-low"
                      }${runningVisual ? " dw-session-row--running" : ""}`}
                    >
                      <span
                        className={`shrink-0 w-2 h-2 rounded-full mt-1.5 ${statusDotClass(s.status, s.trusted_status, runningVisual)}`}
                        title={s.status}
                      />
                      <span className="min-w-0 flex-1">
                        <span className="text-sm font-medium truncate block">
                          {s.title || s.id}
                        </span>
                        {pending > 0 && (
                          <span className="text-xs text-warn truncate block mt-0.5">
                            {t("home.securityPendingBadge").replace("{n}", String(pending))}
                          </span>
                        )}
                      </span>
                      <span className="shrink-0 flex flex-col items-end gap-1 pt-0.5 min-w-[2.75rem]">
                        {runningVisual ? (
                          <SessionRunningDots />
                        ) : (
                          <span className="text-xs text-secondary tabular-nums">
                            {formatRelativeTime(s.started_at)}
                          </span>
                        )}
                        {showAlertBadge && (
                          <SessionStatusBadges
                            variant="sidebar"
                            status={s.status}
                            trustedStatus={s.trusted_status}
                            pendingApprovalCount={pending}
                          />
                        )}
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
  liveBlocks = [],
  liveEvents = [],
  chatStreamLive = false,
  sessionLive,
  questionsRespondAllowed = true,
  approvalsRespondAllowed = true,
  pendingApprovalCount = 0,
  sseStatus = "offline",
  isOptimisticStreaming = false,
  markSessionStreaming,
  clearOptimisticStreaming,
  toolbarStart,
  headerEnd,
  selectedToolId,
  onSelectTool,
  onRenameSession,
}: {
  session: SessionWithProject | null;
  onFollowUpStarted?: (sessionId: string) => void;
  showHeader?: boolean;
  sseLive?: boolean;
  liveBlocks?: import("@/api/types").TranscriptBlock[];
  liveEvents?: import("@/lib/liveTranscript").ChatStreamEvent[];
  chatStreamLive?: boolean;
  sessionLive?: SessionLiveState;
  questionsRespondAllowed?: boolean;
  approvalsRespondAllowed?: boolean;
  /** Disk/summary pending count — used to surface approval UI above composer. */
  pendingApprovalCount?: number;
  sseStatus?: SseStatus;
  isOptimisticStreaming?: boolean;
  markSessionStreaming?: (sessionId: string) => void;
  clearOptimisticStreaming?: () => void;
  toolbarStart?: ReactNode;
  headerEnd?: ReactNode;
  selectedToolId?: string | null;
  onSelectTool?: (tool: import("@/api/types").TranscriptBlock) => void;
  onRenameSession?: (sessionId: string, title: string) => void | Promise<void>;
}) {
  const t = useT();
  const scrollRef = useRef<HTMLDivElement>(null);
  const { sessionSidebarCollapsed, setSessionSidebarCollapsed } = useConversationShell();

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-secondary p-8">
        <Icon name="forum" size={40} className="opacity-40 mb-3" />
        <p className="m-0 text-sm">{t("conversations.selectSession")}</p>
      </div>
    );
  }

  const running = conversationThreadRunning(
    session.status,
    session.id,
    isOptimisticStreaming ? session.id : null,
  );
  const streamLive = conversationStreamLive(chatStreamLive, sseLive, running);
  // Do not treat a stuck `chatStreamLive` alone as active — only session status
  // (or optimistic send) should block the composer after the turn ends.
  const turnActive = running || isOptimisticStreaming;

  const liveLog = useQuery({
    queryKey: ["session-execution-log-live", session.id],
    queryFn: () => api.sessionExecutionLog(session.id, { offset: 0, limit: 120 }),
    enabled: session.status === "running" && !streamLive && sseStatus === "offline",
    staleTime: 30_000,
    refetchInterval:
      session.status === "running" && !streamLive && sseStatus === "offline" ? 30_000 : false,
    refetchIntervalInBackground: false,
  });
  const activeTool = streamLive
    ? null
    : findActiveToolInExecutionLog(liveLog.data?.execution_log.lines ?? []);
  const streamHasActivity = hasTurnStreamActivity(liveBlocks, activeTool);
  const hideComposerWaiting = hideComposerWaitingFromSession({
    running,
    streamHasActivity: streamLive || streamHasActivity,
    turnPhase: sessionLive?.turnPhase ?? null,
    liveBlocksActive: streamHasActivity,
  });

  return (
    <div className={`flex flex-col h-full min-h-0${headerEnd ? " conv-thread--workbench-host" : ""}`}>
      {showHeader && (
        <div className="conv-thread-header bg-surface-container-lowest shrink-0" data-tauri-drag-region>
          <div className="conv-thread-header__row">
            <div className="conv-thread-header__side">
              <button
                type="button"
                className={`dw-btn-ghost p-1.5${sessionSidebarCollapsed ? " text-primary" : ""}`}
                aria-pressed={!sessionSidebarCollapsed}
                aria-label={
                  sessionSidebarCollapsed
                    ? t("conversations.expandSessions")
                    : t("conversations.collapseSessions")
                }
                title={
                  sessionSidebarCollapsed
                    ? t("conversations.expandSessions")
                    : t("conversations.collapseSessions")
                }
                onClick={() => setSessionSidebarCollapsed(!sessionSidebarCollapsed)}
              >
                <Icon name="view_sidebar" size={18} className="scale-x-[-1]" />
              </button>
            </div>
            <div className="conv-thread-header__center">
              <SessionTitleMenu session={session} onRename={onRenameSession} />
            </div>
            <div className="conv-thread-header__side conv-thread-header__side--end">
              {headerEnd}
            </div>
          </div>
        </div>
      )}

      {toolbarStart ? (
        <div className="hidden lg:flex items-center gap-3 px-3 py-2 border-b border-outline-variant bg-surface-container-low shrink-0">
          <div className="shrink-0">{toolbarStart}</div>
        </div>
      ) : null}

      <div
        ref={scrollRef}
        className="conv-thread-transcript-scroll flex-1 overflow-y-auto min-h-0 overscroll-y-contain"
      >
        <div className="conv-thread-body">
          <ConversationTranscript
            sessionId={session.id}
            projectId={session.project_id}
            modelName={session.model}
            isRunning={running}
            streamLive={streamLive}
            sseLive={sseLive}
            sseStatus={sseStatus}
            liveBlocks={liveBlocks}
            liveEvents={liveEvents}
            chatStreamLive={chatStreamLive}
            sessionLive={sessionLive}
            questionsRespondAllowed={questionsRespondAllowed}
            approvalsRespondAllowed={approvalsRespondAllowed}
            scrollContainerRef={scrollRef}
            promptPreview={session.prompt_preview}
            selectedToolId={selectedToolId}
            onSelectTool={onSelectTool}
          />
        </div>
      </div>

      <div className="conv-thread-composer-dock">
        <div className="conv-thread-composer">
          {(sessionLive?.pendingQuestions.length ?? 0) > 0 && (
            <div className="conv-thread-approval-pin px-1 pb-2">
              <AskUserQuestionInbox
                sessionId={session.id}
                hideWhenEmpty
                inline
                questions={sessionLive?.pendingQuestions ?? []}
                respondAllowed={questionsRespondAllowed}
              />
            </div>
          )}
          {(pendingApprovalCount > 0 ||
            (sessionLive?.pendingApprovals.length ?? 0) > 0) && (
            <div className="conv-thread-approval-pin px-1 pb-2">
              <SecurityApprovalInbox
                sessionId={session.id}
                hideWhenEmpty
                inline
                liveApprovals={sessionLive?.pendingApprovals ?? []}
                respondAllowed={approvalsRespondAllowed}
              />
            </div>
          )}
          {session.project_id ? (
            <ConversationGitBar projectId={session.project_id} />
          ) : null}
          <ConversationComposer
            mode="follow-up"
            session={session}
            onSent={onFollowUpStarted}
            hideWaitingIndicator={hideComposerWaiting}
            onStreamingStart={markSessionStreaming}
            onStreamingEnd={clearOptimisticStreaming}
            waitingForQuestion={(sessionLive?.pendingQuestions.length ?? 0) > 0}
            turnActive={turnActive}
            chatStreamLive={chatStreamLive}
            sseStatus={sseStatus}
          />
        </div>
      </div>
    </div>
  );
}
