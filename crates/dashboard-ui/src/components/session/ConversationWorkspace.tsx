import { Link } from "@tanstack/react-router";
import { ConversationThread } from "@/components/ConversationThread";
import { ProjectGroupedSessionList } from "@/components/session/ProjectGroupedSessionList";
import { ConversationWorkbenchSidebar } from "@/components/workbench/ConversationWorkbenchSidebar";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

export function ConversationWorkspace() {
  const t = useT();
  const {
    workbenchDrawerOpen,
    setWorkbenchDrawerOpen,
    sessionsDrawerOpen,
    setSessionsDrawerOpen,
    selectedTool,
    setSelectedTool,
    active,
    rows,
    sidebarFilteredRows,
    listSearch,
    displaySessionId,
    selected,
    selectSession,
    pendingCounts,
    sessionsLoading,
    sessionsError,
    pendingCountsLoading,
    sseLive,
    liveBlocks,
    liveEvents,
    chatStreamLive,
    sessionLive,
    questionsRespondAllowed,
    approvalsRespondAllowed,
    sseStatus,
    isOptimisticStreaming,
    markSessionStreaming,
    clearOptimisticStreaming,
    projectOptions,
    prefetchSession,
    startSessionForProject,
    onRenameSession,
    onRenameProject,
    onRemoveProject,
    optimisticStreamingSessionId,
  } = useConversationShell();

  if (sessionsError) {
    return (
      <div className="dw-alert-error">
        <p className="m-0 font-medium">{t("common.error")}</p>
        <p className="m-0 mt-1 text-sm">{sessionsError.message}</p>
      </div>
    );
  }

  if (sessionsLoading) {
    return <p className="text-sm text-secondary p-4">{t("common.loading")}</p>;
  }

  if (active === "needs_approval" && pendingCountsLoading && rows.length === 0) {
    return <p className="text-sm text-secondary p-4">{t("common.loading")}</p>;
  }

  if (rows.length === 0 && active === "all" && sidebarFilteredRows.length === 0) {
    return (
      <div className="p-6 border border-outline-variant rounded-lg bg-surface-container-lowest m-4">
        <EmptyState
          title={t("conversations.emptyTitle")}
          description={t("conversations.emptyDesc")}
          icon="forum"
        />
        <div className="text-center mt-4">
          <Link to="/" className="dw-btn-primary no-underline inline-flex">
            {t("conversations.newSession")}
          </Link>
        </div>
      </div>
    );
  }

  if (rows.length === 0 && active !== "all" && !selected) {
    return (
      <EmptyState
        title={
          active === "needs_approval"
            ? t("conversations.emptyNeedsApproval")
            : t("conversations.emptyFilter")
        }
        description={
          active === "needs_approval" ? t("conversations.emptyNeedsApprovalDesc") : undefined
        }
        icon="forum"
      />
    );
  }

  if (!selected && rows.length === 0) {
    return null;
  }

  return (
    <>
      <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
        <div className="lg:hidden flex items-center justify-between gap-2 px-3 py-2 border-b border-outline-variant bg-surface-container-low shrink-0">
          <button
            type="button"
            className="dw-btn-secondary text-xs"
            onClick={() => setSessionsDrawerOpen(true)}
          >
            <Icon name="forum" size={16} />
            {t("conversations.sessionList")}
          </button>
        </div>

        <div className="flex flex-1 min-h-0 min-w-0">
          <div className="flex-1 min-h-0 min-w-0 flex flex-col">
            <ConversationThread
              session={selected}
              onFollowUpStarted={selectSession}
              showHeader={true}
              sseLive={sseLive}
              liveBlocks={liveBlocks}
              liveEvents={liveEvents}
              chatStreamLive={chatStreamLive}
              sessionLive={sessionLive}
              questionsRespondAllowed={questionsRespondAllowed}
              approvalsRespondAllowed={approvalsRespondAllowed}
              pendingApprovalCount={
                selected ? (pendingCounts.get(selected.id) ?? 0) : 0
              }
              sseStatus={sseStatus}
              isOptimisticStreaming={isOptimisticStreaming}
              markSessionStreaming={markSessionStreaming}
              clearOptimisticStreaming={clearOptimisticStreaming}
              selectedToolId={selectedTool?.id ?? null}
              onSelectTool={(tool) => {
                setSelectedTool(tool);
                setWorkbenchDrawerOpen(true);
              }}
              onRenameSession={onRenameSession}
              workbenchOpen={workbenchDrawerOpen}
              onToggleWorkbench={() => setWorkbenchDrawerOpen(!workbenchDrawerOpen)}
            />
          </div>

          {workbenchDrawerOpen ? (
            <aside
              className="conv-workbench-dock hidden lg:flex shrink-0 min-h-0 h-full"
              aria-label={t("workbench.title")}
            >
              <ConversationWorkbenchSidebar
                projectId={selected?.project_id}
                sessionId={displaySessionId}
                live={sseLive}
                isRunning={selected?.status === "running"}
                liveBlocks={liveBlocks}
                forceExpanded
                onRequestClose={() => setWorkbenchDrawerOpen(false)}
                className="h-full"
              />
            </aside>
          ) : null}
        </div>
      </div>

      {sessionsDrawerOpen && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-40 bg-black/30 lg:hidden"
            aria-label={t("common.back")}
            onClick={() => setSessionsDrawerOpen(false)}
          />
          <div className="fixed inset-y-0 left-0 z-50 w-[min(100%,20rem)] lg:hidden shadow-xl">
            <div className="h-full border-r border-outline-variant bg-surface-container-lowest flex flex-col">
              <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0 flex items-center justify-between">
                <span>{t("conversations.sessionList")}</span>
                <button
                  type="button"
                  className="dw-btn-ghost p-1"
                  onClick={() => setSessionsDrawerOpen(false)}
                >
                  <Icon name="close" size={18} />
                </button>
              </div>
              <div className="flex-1 min-h-0 overflow-y-auto">
                <ProjectGroupedSessionList
                  projectOptions={projectOptions}
                  sessions={sidebarFilteredRows}
                  selectedId={displaySessionId}
                  onSelect={(id) => {
                    selectSession(id);
                    setSessionsDrawerOpen(false);
                  }}
                  pendingCounts={pendingCounts}
                  onPrefetch={prefetchSession}
                  hideEmptyProjects={listSearch.trim().length > 0}
                  onNewSession={startSessionForProject}
                  onRenameSession={onRenameSession}
                  onRenameProject={onRenameProject}
                  onRemoveProject={onRemoveProject}
                  optimisticStreamingSessionId={optimisticStreamingSessionId}
                />
              </div>
            </div>
          </div>
        </>
      )}

      {/* Mobile: workbench as right edge drawer (push not available on narrow screens). */}
      {workbenchDrawerOpen ? (
        <>
          <button
            type="button"
            className="fixed inset-0 z-40 bg-black/30 lg:hidden border-0 cursor-default"
            aria-label={t("controlCenter.close")}
            onClick={() => setWorkbenchDrawerOpen(false)}
          />
          <div className="fixed inset-y-0 right-0 z-50 w-[min(100%,22rem)] lg:hidden shadow-xl flex bg-surface-container-lowest border-l border-outline-variant">
            <ConversationWorkbenchSidebar
              projectId={selected?.project_id}
              sessionId={displaySessionId}
              live={sseLive}
              isRunning={selected?.status === "running"}
              liveBlocks={liveBlocks}
              forceExpanded
              onRequestClose={() => setWorkbenchDrawerOpen(false)}
              className="h-full w-full"
            />
          </div>
        </>
      ) : null}
    </>
  );
}
