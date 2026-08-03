import { useState } from "react";
import { Icon } from "@/components/Icon";
import { ProjectGroupedSessionList } from "@/components/session/ProjectGroupedSessionList";
import { SessionSearchModal } from "@/components/session/SessionSearchModal";
import { IncomingHandoffBanner } from "@/components/colleagues/IncomingHandoffBanner";
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
  const { openControlCenter, open, activePath } = useControlCenter();
  const {
    sidebarRows,
    sidebarFilteredRows,
    displaySessionId,
    selectSession,
    pendingCounts,
    listBusy,
    projectOptions,
    projectsError,
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
  const colleaguesActive = open && activePath.startsWith("/colleagues");

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
      id: "discover-colleagues",
      labelKey: "sidebar.discoverColleagues",
      icon: "group",
      onClick: () => openControlCenter("/colleagues"),
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
    // Narrow inert rail reserves space for macOS traffic lights only (transparent — no glass strip).
    // Expand / collapse lives in the conversation header (not here — was unclickable under lights).
    return (
      <aside
        className="dw-session-sidebar dw-session-sidebar--collapsed"
        data-tauri-drag-region
        aria-hidden
      />
    );
  }

  return (
    <aside className="dw-session-sidebar glass-panel" data-tauri-drag-region>
      <div className="dw-session-sidebar__header">
        <nav className="dw-sidebar-quick" aria-label={t("sidebar.quickNav")}>
          {quickActions.map((action) => (
            <button
              key={action.id}
              type="button"
              className={`dw-sidebar-quick__item${
                (action.id === "search" && searchOpen) ||
                (action.id === "discover-colleagues" && colleaguesActive)
                  ? " dw-sidebar-quick__item--active"
                  : ""
              }`}
              onClick={action.onClick}
            >
              <Icon name={action.icon} size={16} />
              <span>{t(action.labelKey)}</span>
            </button>
          ))}
        </nav>

        <div className="dw-sidebar-section-label px-3 pt-1 pb-0.5">
          {t("sidebar.sectionProjects")}
        </div>
        {projectsError ? (
          <p className="px-3 text-xs text-warn m-0 mb-1">
            {/\b401\b/.test(projectsError.message)
              ? t("projects.authError")
              : projectsError.message || t("projects.loadError")}
          </p>
        ) : null}
      </div>

      <IncomingHandoffBanner />

      <div
        className={`dw-session-sidebar__scroll flex-1 min-h-0 overflow-y-auto overscroll-y-contain transition-opacity ${listBusy ? "opacity-60" : ""}`}
      >
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
