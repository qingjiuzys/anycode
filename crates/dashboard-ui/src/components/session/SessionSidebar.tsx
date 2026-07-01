import { Link } from "@tanstack/react-router";
import { ConversationSessionList } from "@/components/ConversationThread";
import { AppearanceMenu } from "@/components/AppearanceMenu";
import { BrandMark } from "@/components/BrandMark";
import { Icon } from "@/components/Icon";
import { ProjectPicker } from "@/components/ProjectPicker";
import { SidebarWorkspaceCard } from "@/components/SidebarWorkspaceCard";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

export function SessionSidebar({
  collapsed,
  onCollapsedChange,
}: {
  collapsed: boolean;
  onCollapsedChange: (collapsed: boolean) => void;
}) {
  const t = useT();
  const {
    projectId,
    setProjectId,
    setShowStartForm,
    active,
    quickChips,
    applyChip,
    listSearch,
    setListSearch,
    filteredRows,
    displaySessionId,
    selectSession,
    pendingCounts,
    listBusy,
    navigateSearch,
    effectiveSearch,
    projectOptions,
    prefetchSession,
  } = useConversationShell();

  const setProjectAndNavigate = (nextProject: string) => {
    setProjectId(nextProject);
    navigateSearch({
      ...effectiveSearch,
      project: nextProject || undefined,
      session: undefined,
    });
  };

  if (collapsed) {
    return (
      <aside className="dw-session-sidebar dw-session-sidebar--collapsed glass-panel">
        <div className="dw-session-sidebar__drag" aria-hidden />
        <button
          type="button"
          className="dw-sidebar-icon-button"
          aria-label={t("common.expand")}
          title={t("common.expand")}
          onClick={() => onCollapsedChange(false)}
        >
          <Icon name="chevron_right" size={18} />
        </button>
        <Link to="/conversations" className="dw-sidebar-icon-button no-underline" title={t("nav.conversations")}>
          <Icon name="chat" size={18} />
        </Link>
        <button
          type="button"
          className="dw-sidebar-icon-button"
          title={t("conversations.newSession")}
          onClick={() => setShowStartForm((v) => !v)}
        >
          <Icon name="add" size={18} />
        </button>
        <Link to="/projects" className="dw-sidebar-icon-button no-underline" title={t("nav.projects")}>
          <Icon name="folder" size={18} />
        </Link>
        <div className="dw-sidebar-collapsed-spacer" />
        <AppearanceMenu compact />
        <Link
          to="/settings"
          search={{ section: "prefs" }}
          className="dw-sidebar-icon-button no-underline"
          title={t("nav.settings")}
        >
          <Icon name="settings" size={18} />
        </Link>
      </aside>
    );
  }

  return (
    <aside className="dw-session-sidebar glass-panel">
      <div className="dw-session-sidebar__drag" aria-hidden />
      <div className="dw-sidebar-brand dw-session-sidebar__brand">
        <BrandMark size="md" showTitle linked homeTo="/conversations" />
        <button
          type="button"
          className="dw-sidebar-icon-button"
          aria-label={t("common.collapse")}
          title={t("common.collapse")}
          onClick={() => onCollapsedChange(true)}
        >
          <Icon name="chevron_left" size={18} />
        </button>
      </div>

      <SidebarWorkspaceCard />

      <div className="dw-session-sidebar-filters px-2 pb-2 space-y-2 shrink-0">
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
        <ProjectPicker
          value={projectId}
          options={projectOptions}
          includeAll
          className="w-full"
          buttonClassName="w-full"
          onChange={setProjectAndNavigate}
        />
        <div className="dw-session-primary-links" aria-label={t("nav.projects")}>
          <Link to="/projects" className="dw-session-primary-link no-underline">
            <Icon name="folder" size={16} />
            {t("nav.projects")}
          </Link>
          <Link
            to="/conversations"
            search={{ project: projectId || undefined }}
            className="dw-session-primary-link no-underline"
          >
            <Icon name="chat" size={16} />
            {t("nav.conversations")}
          </Link>
        </div>
        <div className="flex flex-wrap gap-1">
          {quickChips.map((f) => (
            <button
              key={f.id}
              type="button"
              className={`dw-chip text-xs${active === f.id ? " active" : ""}`}
              onClick={() => applyChip(f.id)}
            >
              {f.label}
              {f.badge != null && Number(f.badge) > 0 && (
                <span className="ml-1 rounded-full bg-warn/20 text-warn px-1 text-[10px]">
                  {f.badge}
                </span>
              )}
            </button>
          ))}
        </div>
        {projectId && (
          <button
            type="button"
            className="dw-btn-primary w-full text-sm"
            onClick={() => setShowStartForm((v) => !v)}
          >
            <Icon name="add" size={16} />
            {t("conversations.newSession")}
          </button>
        )}
      </div>

      <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-y border-outline-variant bg-surface-container-low shrink-0">
        {t("conversations.sessionList")}
        {filteredRows.length > 0 && ` (${filteredRows.length})`}
      </div>

      <div
        className={`flex-1 min-h-0 overflow-y-auto overscroll-y-contain transition-opacity ${listBusy ? "opacity-60 pointer-events-none" : ""}`}
      >
        <ConversationSessionList
          sessions={filteredRows}
          selectedId={displaySessionId}
          onSelect={selectSession}
          pendingCounts={pendingCounts}
          onPrefetch={prefetchSession}
        />
      </div>

      <div className="dw-session-sidebar-footer">
        <AppearanceMenu />
        <Link
          to="/settings"
          search={{ section: "prefs" }}
          className="dw-nav-link no-underline"
        >
          <Icon name="settings" size={18} />
          <span>{t("nav.settings")}</span>
        </Link>
      </div>

    </aside>
  );
}
