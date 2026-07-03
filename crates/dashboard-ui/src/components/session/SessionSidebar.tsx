import { BrandMark } from "@/components/BrandMark";
import { Icon } from "@/components/Icon";
import { ProjectGroupedSessionList } from "@/components/session/ProjectGroupedSessionList";
import { SidebarFooter } from "@/components/SidebarFooter";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

export function SessionSidebar() {
  const t = useT();
  const {
    listSearch,
    setListSearch,
    sidebarFilteredRows,
    displaySessionId,
    selectSession,
    pendingCounts,
    listBusy,
    projectOptions,
    prefetchSession,
    startSessionForProject,
  } = useConversationShell();

  return (
    <aside className="dw-session-sidebar glass-panel">
      <div className="dw-sidebar-brand">
        <BrandMark size="md" showTitle linked homeTo="/" />
      </div>

      <div className="dw-session-sidebar-search px-2 pb-2 shrink-0">
        <div className="relative">
          <Icon
            name="search"
            size={16}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-outline pointer-events-none"
          />
          <input
            type="search"
            className="dw-input w-full pl-9 text-sm"
            placeholder={t("conversations.sessionSearch")}
            value={listSearch}
            onChange={(e) => setListSearch(e.target.value)}
          />
        </div>
      </div>

      <div
        className={`flex-1 min-h-0 overflow-y-auto overscroll-y-contain transition-opacity ${listBusy ? "opacity-60 pointer-events-none" : ""}`}
      >
        <ProjectGroupedSessionList
          projectOptions={projectOptions}
          sessions={sidebarFilteredRows}
          selectedId={displaySessionId}
          onSelect={selectSession}
          pendingCounts={pendingCounts}
          onPrefetch={prefetchSession}
          hideEmptyProjects={listSearch.trim().length > 0}
          onNewSession={startSessionForProject}
        />
      </div>

      <SidebarFooter />
    </aside>
  );
}
