import type { ReactNode } from "react";
import type { WorkbenchTab } from "@/api/types/workbench";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

const TAB_TITLE: Record<WorkbenchTab, string> = {
  files: "workbench.tabFiles",
  browser: "workbench.tabBrowser",
  terminal: "workbench.tabTerminal",
  artifacts: "workbench.tabArtifacts",
  plan: "workbench.tabPlan",
};

type Props = {
  activeTab: WorkbenchTab;
  width: number;
  onResizeStart: (e: React.PointerEvent) => void;
  onCollapse: () => void;
  children: ReactNode;
};

export function WorkbenchPanel({
  activeTab,
  width: _width,
  onResizeStart,
  onCollapse,
  children,
}: Props) {
  const t = useT();

  return (
    <div
      className="conv-workbench-panel relative flex flex-col min-h-0 border-l border-outline-variant bg-surface-container-lowest w-full min-w-0"
      style={{ flex: "1 1 auto" }}
    >
      <div
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30 z-10 dw-no-drag"
        onPointerDown={onResizeStart}
        data-no-window-drag
        aria-hidden
      />
      <div
        className="px-3 py-2.5 text-sm font-semibold text-secondary border-b border-outline-variant bg-surface-container-low shrink-0 flex items-center justify-between gap-2"
        data-tauri-drag-region
      >
        <span className="inline-flex items-center gap-1.5">
          <Icon name="view_sidebar" size={16} />
          {t(TAB_TITLE[activeTab] as "workbench.tabFiles")}
        </span>
        <button
          type="button"
          className="dw-btn-ghost p-1"
          title={t("workbench.collapse")}
          aria-label={t("workbench.collapse")}
          onClick={onCollapse}
        >
          <Icon name="close" size={18} />
        </button>
      </div>
      <div className="flex-1 min-h-[12rem] flex flex-col overflow-y-auto overflow-x-hidden">
        {children}
      </div>
    </div>
  );
}
