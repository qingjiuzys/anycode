/** Built-in agent ids aligned with CLI subagents / dashboard stats. */
export const BUILTIN_AGENT_CATALOG = [
  { id: "general-purpose", icon: "smart_toy", labelKey: "generalPurpose" },
  { id: "explore", icon: "travel_explore", labelKey: "explore" },
  { id: "plan", icon: "psychology", labelKey: "plan" },
  { id: "workspace-assistant", icon: "hub", labelKey: "workspaceAssistant" },
  { id: "goal", icon: "flag", labelKey: "goal" },
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
  "builder",
  "goal-runner",
] as const;

export type BuiltinAgentId = (typeof BUILTIN_AGENT_CATALOG)[number]["id"];

export function isPrimaryAgentId(id: string): boolean {
  return (PRIMARY_AGENT_IDS as readonly string[]).includes(id);
}

export function builtinAgentMeta(id: string) {
  return BUILTIN_AGENT_CATALOG.find((a) => a.id === id);
}
