import type { ReactNode } from "react";
import type { WorkbenchTab } from "@/api/types/workbench";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

const TAB_TITLE: Record<WorkbenchTab, string> = {
  files: "workbench.tabFiles",
  browser: "workbench.tabBrowser",
  terminal: "workbench.tabTerminal",
  artifacts: "workbench.tabArtifacts",
  trace: "workbench.tabTrace",
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
  width,
  onResizeStart,
  onCollapse,
  children,
}: Props) {
  const t = useT();

  return (
    <div
      className="conv-workbench-panel relative flex flex-col min-h-0 shrink-0 border-l border-outline-variant bg-surface-container-lowest"
      style={{ width }}
    >
      <div
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30 z-10"
        onPointerDown={onResizeStart}
        aria-hidden
      />
      <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0 flex items-center justify-between gap-2">
        <span className="inline-flex items-center gap-1.5">
          <Icon name="view_sidebar" size={14} />
          {t(TAB_TITLE[activeTab] as "workbench.tabFiles")}
        </span>
        <button
          type="button"
          className="dw-btn-ghost p-1"
          title={t("workbench.collapse")}
          onClick={onCollapse}
        >
          <Icon name="chevron_right" size={18} />
        </button>
      </div>
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">{children}</div>
    </div>
  );
}
