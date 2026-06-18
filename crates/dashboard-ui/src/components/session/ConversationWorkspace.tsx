import { ConversationSessionList, ConversationThread } from "@/components/ConversationThread";
import { ConversationWorkbenchSidebar } from "@/components/workbench/ConversationWorkbenchSidebar";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

export function ConversationWorkspace() {
  const t = useT();
  const {
    projectId,
    showStartForm,
    setShowStartForm,
    workbenchDrawerOpen,
    setWorkbenchDrawerOpen,
    sessionsDrawerOpen,
    setSessionsDrawerOpen,
    selectedTool,
    setSelectedTool,
    active,
    rows,
    displaySessionId,
    selected,
    selectSession,
    pendingCounts,
    sessionsLoading,
    sessionsError,
    pendingCountsLoading,
    sseLive,
    renderStartComposer,
    prefetchSession,
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

  if (!projectId && rows.length === 0 && active === "all") {
    return (
      <EmptyState
        title={t("conversations.selectProjectFirst")}
        description={t("conversations.selectProjectFirstDesc")}
        icon="folder_open"
      />
    );
  }

  if (projectId && rows.length === 0 && active === "all") {
    return (
      <div className="p-6 border border-outline-variant rounded-lg bg-surface-container-lowest m-4">
        {!showStartForm && (
          <EmptyState
            title={t("conversations.emptyTitle")}
            description={t("conversations.emptyDesc")}
            icon="forum"
          />
        )}
        {showStartForm ? (
          renderStartComposer()
        ) : (
          <div className="text-center mt-4">
            <button type="button" className="dw-btn-primary" onClick={() => setShowStartForm(true)}>
              {t("conversations.newSession")}
            </button>
          </div>
        )}
      </div>
    );
  }

  if (rows.length === 0 && active !== "all") {
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

  if (rows.length === 0) {
    return null;
  }

  return (
    <>
      {projectId && showStartForm && (
        <div className="shrink-0 p-3 border-b border-outline-variant bg-surface-container-low">
          {renderStartComposer(true)}
        </div>
      )}

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
          <button
            type="button"
            className="dw-btn-secondary text-xs"
            onClick={() => setWorkbenchDrawerOpen(true)}
          >
            <Icon name="view_sidebar" size={16} />
            {t("workbench.title")}
          </button>
        </div>

        <div className="flex flex-1 min-h-0">
          <div className="flex-1 min-h-0 flex flex-col min-w-0">
            <ConversationThread
              session={selected}
              onFollowUpStarted={selectSession}
              showHeader={false}
              sseLive={sseLive}
              selectedToolId={selectedTool?.id ?? null}
              onSelectTool={(tool) => {
                setSelectedTool(tool);
                if (window.matchMedia("(max-width: 1023px)").matches) {
                  setWorkbenchDrawerOpen(true);
                }
              }}
            />
          </div>
          <div className="hidden lg:flex shrink-0 min-h-0">
            <ConversationWorkbenchSidebar
              projectId={selected?.project_id}
              sessionId={displaySessionId}
              live={sseLive}
              isRunning={selected?.status === "running"}
              selectedTool={selectedTool}
              onSelectTool={setSelectedTool}
            />
          </div>
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
                <ConversationSessionList
                  sessions={rows}
                  selectedId={displaySessionId}
                  onSelect={(id) => {
                    selectSession(id);
                    setSessionsDrawerOpen(false);
                  }}
                  pendingCounts={pendingCounts}
                  onPrefetch={prefetchSession}
                />
              </div>
            </div>
          </div>
        </>
      )}

      {workbenchDrawerOpen && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-40 bg-black/30 lg:hidden"
            aria-label={t("common.back")}
            onClick={() => setWorkbenchDrawerOpen(false)}
          />
          <div className="fixed inset-y-0 right-0 z-50 w-[min(100%,22rem)] lg:hidden shadow-xl flex">
            <ConversationWorkbenchSidebar
              projectId={selected?.project_id}
              sessionId={displaySessionId}
              live={sseLive}
              isRunning={selected?.status === "running"}
              selectedTool={selectedTool}
              onSelectTool={setSelectedTool}
              forceExpanded
              className="h-full border-l border-outline-variant bg-surface-container-lowest"
            />
            <button
              type="button"
              className="absolute top-2 right-14 dw-btn-ghost p-1.5 z-10"
              onClick={() => setWorkbenchDrawerOpen(false)}
            >
              <Icon name="close" size={18} />
            </button>
          </div>
        </>
      )}
    </>
  );
}