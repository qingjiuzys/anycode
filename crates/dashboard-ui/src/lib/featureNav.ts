/** Global feature navigation (formerly permanent left sidebar). */

export type FeatureCountKey = "projects" | "sessions" | "artifacts" | "skills";

export type FeatureNavGroup = "workspace" | "automation" | "ops" | "agents" | "config" | "external";

export type FeatureNavItem = {
  id: string;
  to: string;
  /** i18n key under nav.* */
  key: string;
  icon: string;
  countKey?: FeatureCountKey | null;
  group: FeatureNavGroup;
};

export const FEATURE_NAV_GROUPS: { id: FeatureNavGroup; labelKey: string }[] = [
  { id: "workspace", labelKey: "controlCenter.groupWorkspace" },
  { id: "automation", labelKey: "controlCenter.groupAutomation" },
  { id: "ops", labelKey: "controlCenter.groupOps" },
  { id: "agents", labelKey: "controlCenter.groupAgents" },
  { id: "config", labelKey: "controlCenter.groupConfig" },
  { id: "external", labelKey: "controlCenter.groupExternal" },
];

export const FEATURE_NAV: FeatureNavItem[] = [
  { id: "home", to: "/", key: "nav.home", icon: "home", group: "workspace" },
  { id: "overview", to: "/overview", key: "nav.overview", icon: "dashboard", group: "workspace" },
  {
    id: "projects",
    to: "/projects",
    key: "nav.projects",
    icon: "folder",
    countKey: "projects",
    group: "workspace",
  },
  {
    id: "automations",
    to: "/automations",
    key: "nav.automations",
    icon: "settings_suggest",
    group: "automation",
  },
  {
    id: "assets",
    to: "/assets",
    key: "nav.assets",
    icon: "inventory_2",
    countKey: "artifacts",
    group: "ops",
  },
  { id: "reports", to: "/reports", key: "nav.reports", icon: "bar_chart", group: "ops" },
  { id: "audit", to: "/audit", key: "nav.audit", icon: "verified_user", group: "ops" },
  { id: "account", to: "/account", key: "nav.account", icon: "corporate_fare", group: "config" },
  {
    id: "agents",
    to: "/agents",
    key: "nav.agents",
    icon: "robot_2",
    countKey: "skills",
    group: "agents",
  },
  { id: "settings", to: "/settings", key: "nav.settings", icon: "settings", group: "config" },
];

export function navCount(
  key: FeatureCountKey | null | undefined,
  ov?: {
    projects_count: number;
    sessions_total: number;
    artifacts_count: number;
    skills_count: number;
  },
): number | null {
  if (!key || !ov) return null;
  switch (key) {
    case "projects":
      return ov.projects_count;
    case "sessions":
      return ov.sessions_total;
    case "artifacts":
      return ov.artifacts_count;
    case "skills":
      return ov.skills_count;
    default:
      return null;
  }
}

export function featureNavByPath(path: string): FeatureNavItem | undefined {
  const pathname = path.split("?")[0] ?? path;
  if (pathname === "/") return FEATURE_NAV.find((item) => item.to === "/");
  return FEATURE_NAV.find(
    (item) => item.to !== "/" && (pathname === item.to || pathname.startsWith(`${item.to}/`)),
  );
}
