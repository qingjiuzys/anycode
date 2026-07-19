import { useState } from "react";
import { BrandMark } from "@/components/BrandMark";
import { Icon } from "@/components/Icon";
import { ProjectGroupedSessionList } from "@/components/session/ProjectGroupedSessionList";
import { SessionSearchModal } from "@/components/session/SessionSearchModal";
import { SidebarFooter } from "@/components/SidebarFooter";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

type QuickAction = {
  id: string;
  labelKey: string;
  icon: string;
  onClick: () => void;
};

export function SessionSidebar() {
  const t = useT();
  const { openControlCenter } = useControlCenter();
  const {
    sidebarRows,
    sidebarFilteredRows,
    displaySessionId,
    selectSession,
    pendingCounts,
    listBusy,
    projectOptions,
    projectId,
    prefetchSession,
    startSessionForProject,
    goHome,
    onRenameSession,
    onRenameProject,
    onRemoveProject,
    optimisticStreamingSessionId,
    sessionSidebarCollapsed,
  } = useConversationShell();

  const [searchOpen, setSearchOpen] = useState(false);

  const quickActions: QuickAction[] = [
    {
      id: "new-agent",
      labelKey: "sidebar.newAgent",
      icon: "edit",
      onClick: () => {
        if (projectId) {
          startSessionForProject(projectId);
        } else {
          goHome();
        }
      },
    },
    {
      id: "search",
      labelKey: "sidebar.search",
      icon: "search",
      onClick: () => setSearchOpen(true),
    },
    {
      id: "automations",
      labelKey: "sidebar.automations",
      icon: "schedule",
      onClick: () => openControlCenter("/automations"),
    },
    {
      id: "plugins",
      labelKey: "sidebar.plugins",
      icon: "extension",
      onClick: () => openControlCenter("/settings?section=plugins"),
    },
  ];

  if (sessionSidebarCollapsed) {
    // Narrow inert rail reserves space for macOS traffic lights only.
    // Expand / collapse lives in the conversation header (not here — was unclickable under lights).
    return (
      <aside
        className="dw-session-sidebar glass-panel dw-session-sidebar--collapsed"
        aria-hidden
      />
    );
  }

  return (
    <aside className="dw-session-sidebar glass-panel">
      <div className="dw-sidebar-brand">
        <BrandMark size="md" showTitle linked homeTo="/" />
      </div>

      <nav className="dw-sidebar-quick" aria-label={t("sidebar.quickNav")}>
        {quickActions.map((action) => (
          <button
            key={action.id}
            type="button"
            className={`dw-sidebar-quick__item${
              action.id === "search" && searchOpen ? " dw-sidebar-quick__item--active" : ""
            }`}
            onClick={action.onClick}
          >
            <Icon name={action.icon} size={18} />
            <span>{t(action.labelKey)}</span>
          </button>
        ))}
      </nav>

      <div
        className={`flex-1 min-h-0 overflow-y-auto overscroll-y-contain transition-opacity ${listBusy ? "opacity-60" : ""}`}
      >
        <div className="dw-sidebar-section-label px-3 pt-2 pb-1">
          {t("sidebar.sectionProjects")}
        </div>
        <ProjectGroupedSessionList
          projectOptions={projectOptions}
          sessions={sidebarFilteredRows}
          selectedId={displaySessionId}
          onSelect={selectSession}
          pendingCounts={pendingCounts}
          onPrefetch={prefetchSession}
          onNewSession={startSessionForProject}
          activeProjectId={projectId}
          onSelectProject={goHome}
          onRenameSession={onRenameSession}
          onRenameProject={onRenameProject}
          onRemoveProject={onRemoveProject}
          optimisticStreamingSessionId={optimisticStreamingSessionId}
        />
      </div>

      <SidebarFooter />

      <SessionSearchModal
        open={searchOpen}
        onClose={() => setSearchOpen(false)}
        sessions={sidebarRows.length > 0 ? sidebarRows : sidebarFilteredRows}
        onSelect={selectSession}
      />
    </aside>
  );
}
