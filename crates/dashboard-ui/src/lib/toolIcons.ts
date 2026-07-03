const TOOL_ICON_BY_NAME: Record<string, string> = {
  Bash: "terminal",
  TodoWrite: "checklist",
  Read: "description",
  Write: "edit_note",
  Edit: "edit",
  Grep: "search",
  Glob: "folder",
  Delete: "delete",
  Task: "task_alt",
  WebSearch: "travel_explore",
  WebFetch: "language",
  Browser: "public",
  StrReplace: "find_replace",
  Shell: "terminal",
  AskQuestion: "help",
};

/** Material icon name for a tool id (falls back to `build`). */
export function toolIconName(toolName: string | undefined): string {
  if (!toolName?.trim()) return "build";
  const direct = TOOL_ICON_BY_NAME[toolName.trim()];
  if (direct) return direct;
  const normalized = toolName.replace(/^mcp__[^_]+__/, "");
  return TOOL_ICON_BY_NAME[normalized] ?? "build";
}
