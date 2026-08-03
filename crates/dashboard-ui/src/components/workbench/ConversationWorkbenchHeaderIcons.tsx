import type { WorkbenchTab } from "@/api/types/workbench";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

const TABS: { id: WorkbenchTab; icon: string; titleKey: string }[] = [
  { id: "files", icon: "folder", titleKey: "workbench.tabFiles" },
  { id: "browser", icon: "language", titleKey: "workbench.tabBrowser" },
  { id: "terminal", icon: "terminal", titleKey: "workbench.tabTerminal" },
  { id: "plan", icon: "account_tree", titleKey: "workbench.tabPlan" },
  { id: "artifacts", icon: "inventory_2", titleKey: "workbench.tabArtifacts" },
];

type Props = {
  activeTab: WorkbenchTab;
  expanded: boolean;
  onSelectTab: (tab: WorkbenchTab) => void;
  disabled?: boolean;
};

export function ConversationWorkbenchHeaderIcons({
  activeTab,
  expanded,
  onSelectTab,
  disabled,
}: Props) {
  const t = useT();

  return (
    <div className="conv-workbench-header-icons" role="toolbar" aria-label={t("workbench.title")}>
      {TABS.map((tab) => {
        const active = expanded && activeTab === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            title={t(tab.titleKey as "workbench.tabFiles")}
            disabled={disabled}
            aria-pressed={active}
            className={`conv-workbench-header-icons__btn${active ? " conv-workbench-header-icons__btn--active" : ""}`}
            onClick={() => onSelectTab(tab.id)}
          >
            <Icon name={tab.icon} size={18} />
          </button>
        );
      })}
    </div>
  );
}
