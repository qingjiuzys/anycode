import { ConversationSessionList } from "@/components/ConversationThread";
import { BrandMark } from "@/components/BrandMark";
import { Icon } from "@/components/Icon";
import { SidebarWorkspaceCard } from "@/components/SidebarWorkspaceCard";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

export function SessionSidebar() {
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

  return (
    <aside className="dw-session-sidebar glass-panel">
      <div className="dw-sidebar-brand">
        <BrandMark size="md" showTitle linked homeTo="/conversations" />
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
        <select
          className="dw-input w-full text-sm"
          value={projectId}
          onChange={(e) => {
            const nextProject = e.target.value;
            setProjectId(nextProject);
            navigateSearch({
              ...effectiveSearch,
              project: nextProject || undefined,
              session: undefined,
            });
          }}
        >
          <option value="">{t("conversations.allProjects")}</option>
          {projectOptions.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
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

    </aside>
  );
}
