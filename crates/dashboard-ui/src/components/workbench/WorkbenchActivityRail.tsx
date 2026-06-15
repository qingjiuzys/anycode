import type { WorkbenchTab } from "@/api/types/workbench";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

const TABS: { id: WorkbenchTab; icon: string; titleKey: string }[] = [
  { id: "files", icon: "folder", titleKey: "workbench.tabFiles" },
  { id: "browser", icon: "language", titleKey: "workbench.tabBrowser" },
  { id: "terminal", icon: "terminal", titleKey: "workbench.tabTerminal" },
  { id: "artifacts", icon: "inventory_2", titleKey: "workbench.tabArtifacts" },
  { id: "trace", icon: "timeline", titleKey: "workbench.tabTrace" },
];

type Props = {
  activeTab: WorkbenchTab;
  onSelectTab: (tab: WorkbenchTab) => void;
  disabled?: boolean;
};

export function WorkbenchActivityRail({ activeTab, onSelectTab, disabled }: Props) {
  const t = useT();

  return (
    <aside className="conv-workbench-rail flex flex-col items-center py-2 gap-1 shrink-0 w-12 border-l border-outline-variant bg-surface-container-low">
      {TABS.map((tab) => {
        const active = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            title={t(tab.titleKey as "workbench.tabFiles")}
            disabled={disabled}
            className={`p-2 rounded-lg border-0 cursor-pointer transition-colors ${
              active
                ? "bg-primary/15 text-primary"
                : "bg-transparent text-secondary hover:bg-surface-container-high hover:text-on-surface"
            } ${disabled ? "opacity-40 cursor-not-allowed" : ""}`}
            onClick={() => onSelectTab(tab.id)}
          >
            <Icon name={tab.icon} size={20} />
          </button>
        );
      })}
    </aside>
  );
}
