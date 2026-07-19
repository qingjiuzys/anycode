/** Built-in agent ids aligned with CLI subagents / dashboard stats. */
export const BUILTIN_AGENT_CATALOG = [
  { id: "general-purpose", icon: "smart_toy", labelKey: "generalPurpose" },
  { id: "explore", icon: "travel_explore", labelKey: "explore" },
  { id: "plan", icon: "psychology", labelKey: "plan" },
  { id: "goal", icon: "flag", labelKey: "goal" },
  { id: "workspace-assistant", icon: "hub", labelKey: "workspaceAssistant" },
  { id: "verifier", icon: "fact_check", labelKey: "verifier" },
  { id: "reviewer", icon: "rate_review", labelKey: "reviewer" },
  { id: "office-writer", icon: "edit_note", labelKey: "officeWriter" },
  { id: "data-analyst", icon: "table_chart", labelKey: "dataAnalyst" },
  { id: "researcher", icon: "science", labelKey: "researcher" },
  { id: "file-operator", icon: "folder_open", labelKey: "fileOperator" },
] as const;

/** Shown first in composer agent picker; others grouped under “More”. */
export const PRIMARY_AGENT_IDS = [
  "general-purpose",
  "explore",
  "plan",
  "goal",
] as const;

/** Legacy ids kept for session labels and routing config compatibility. */
export const DEPRECATED_AGENT_ALIASES: Record<string, string> = {
  builder: "general-purpose",
  planner: "plan",
  explorer: "explore",
  "goal-runner": "goal",
  "channel-ops": "workspace-assistant",
};

/** Localized display names for shipped + builtin agent profile ids. */
export const AGENT_LABEL_KEYS: Record<string, string> = {
  "general-purpose": "generalPurpose",
  explore: "explore",
  plan: "plan",
  "workspace-assistant": "workspaceAssistant",
  goal: "goal",
  builder: "builder",
  planner: "planner",
  explorer: "explorer",
  verifier: "verifier",
  reviewer: "reviewer",
  "channel-ops": "channelOps",
  "goal-runner": "goalRunner",
  "office-writer": "officeWriter",
  "data-analyst": "dataAnalyst",
  researcher: "researcher",
  "file-operator": "fileOperator",
};

export type BuiltinAgentId = (typeof BUILTIN_AGENT_CATALOG)[number]["id"];

export function normalizeAgentId(id: string): string {
  const trimmed = id.trim();
  return DEPRECATED_AGENT_ALIASES[trimmed] ?? trimmed;
}

export function isPrimaryAgentId(id: string): boolean {
  return (PRIMARY_AGENT_IDS as readonly string[]).includes(normalizeAgentId(id));
}

export function agentLabelKey(id: string): string | null {
  const key = AGENT_LABEL_KEYS[id.trim()];
  return key ?? null;
}

/** Friendly localized label for an agent profile id (falls back to raw id). */
export function agentDisplayLabel(id: string, t: (key: string) => string): string {
  const labelKey = agentLabelKey(id);
  if (!labelKey) return id;
  const label = t(`agents.builtin.${labelKey}`);
  return label === `agents.builtin.${labelKey}` ? id : label;
}

export function builtinAgentMeta(id: string) {
  const canonical = normalizeAgentId(id);
  return BUILTIN_AGENT_CATALOG.find((a) => a.id === canonical);
}
