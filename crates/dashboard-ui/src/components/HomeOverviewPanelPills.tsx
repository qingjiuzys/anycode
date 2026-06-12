import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export type HomeOverviewPanelId = "recent" | "analytics" | "workbench";

function panelIcon(id: HomeOverviewPanelId): string {
  switch (id) {
    case "recent":
      return "history";
    case "analytics":
      return "analytics";
    case "workbench":
      return "dashboard_customize";
  }
}

export function HomeOverviewPanelPills({
  activePanelId,
  onPanelChange,
  showRecentPanel,
}: {
  activePanelId: string | null;
  onPanelChange: (id: HomeOverviewPanelId | null) => void;
  showRecentPanel: boolean;
}) {
  const t = useT();

  const pills: { id: HomeOverviewPanelId; label: string }[] = [
    ...(showRecentPanel ? [{ id: "recent" as const, label: t("home.recentSessions") }] : []),
    { id: "analytics", label: t("home.analyticsSection") },
    { id: "workbench", label: t("home.workbenchSection") },
  ];

  return (
    <div className="dw-overview-pills">
      {pills.map((pill) => (
        <button
          key={pill.id}
          type="button"
          className={`dw-overview-pill ${activePanelId === pill.id ? "active" : ""}`}
          onClick={() => onPanelChange(activePanelId === pill.id ? null : pill.id)}
        >
          <Icon name={panelIcon(pill.id)} size={14} />
          {pill.label}
        </button>
      ))}
    </div>
  );
}
