import type { TranscriptBlock } from "@/api/types";
import { formatDurationMs } from "@/lib/toolStepLabel";
import type { ToolStep } from "@/lib/transcriptGrouping";

const EXPLORE_TOOLS = new Set([
  "Glob",
  "Grep",
  "Read",
  "FileRead",
  "WebFetch",
  "WebSearch",
]);

const EDIT_TOOLS = new Set(["Edit", "Write", "StrReplace", "ApplyPatch", "NotebookEdit"]);

export type ActivityCounts = {
  files: number;
  searches: number;
  commands: number;
  editedFiles: number;
  linesAdded: number;
  linesRemoved: number;
};

function toolNameFromStep(step: ToolStep): string {
  const primary = step.result ?? step.call;
  if (!primary) return "";
  const meta = primary.meta?.name;
  if (typeof meta === "string" && meta.trim()) {
    return meta.trim();
  }
  return primary.title.replace(/\s+(started|finished|failed)$/i, "").trim();
}

function metaNumber(meta: Record<string, unknown>, key: string): number {
  const raw = meta[key];
  if (typeof raw === "number" && !Number.isNaN(raw)) return Math.max(0, raw);
  if (typeof raw === "string") {
    const n = Number.parseInt(raw, 10);
    return Number.isNaN(n) ? 0 : Math.max(0, n);
  }
  return 0;
}

function countFromBody(name: string, body: string): Partial<ActivityCounts> {
  const out: Partial<ActivityCounts> = {};
  const lines = body.split("\n").filter((l) => l.trim().length > 0);
  if (name === "Glob") {
    out.files = lines.length;
  } else if (name === "Grep") {
    const matchLine = lines.find((l) => /match/i.test(l));
    if (matchLine) {
      const m = matchLine.match(/(\d+)/);
      if (m) out.searches = Number.parseInt(m[1]!, 10);
    } else {
      out.searches = lines.length;
    }
  } else if (name === "Read" || name === "FileRead") {
    out.files = 1;
  } else if (name === "Bash" || name === "Shell") {
    out.commands = 1;
  }
  return out;
}

export function accumulateActivityCounts(steps: ToolStep[]): ActivityCounts {
  const counts: ActivityCounts = {
    files: 0,
    searches: 0,
    commands: 0,
    editedFiles: 0,
    linesAdded: 0,
    linesRemoved: 0,
  };

  for (const step of steps) {
    const name = toolNameFromStep(step);
    const block = step.result ?? step.call;
    if (!block) continue;
    const meta = (block.meta ?? {}) as Record<string, unknown>;
    const body = block.body ?? "";

    if (EXPLORE_TOOLS.has(name)) {
      counts.files += metaNumber(meta, "files_matched") || countFromBody(name, body).files || 0;
      counts.searches +=
        metaNumber(meta, "matches") ||
        metaNumber(meta, "match_count") ||
        countFromBody(name, body).searches ||
        0;
    }
    if (name === "Bash" || name === "Shell") {
      counts.commands += metaNumber(meta, "commands_run") || 1;
    }
    if (EDIT_TOOLS.has(name)) {
      counts.editedFiles += metaNumber(meta, "files_edited") || 1;
      counts.linesAdded += metaNumber(meta, "lines_added");
      counts.linesRemoved += metaNumber(meta, "lines_removed");
    }
  }

  return counts;
}

export function collectToolNames(steps: ToolStep[]): string[] {
  const seen = new Set<string>();
  const names: string[] = [];
  for (const step of steps) {
    const name = toolNameFromStep(step);
    if (!name || seen.has(name)) continue;
    seen.add(name);
    names.push(name);
  }
  return names;
}

export function totalDurationMs(steps: ToolStep[]): number {
  return steps.reduce((acc, step) => {
    const meta = (step.result?.meta ?? step.call?.meta ?? {}) as Record<string, unknown>;
    const raw = meta.duration_ms ?? meta.elapsed_ms;
    const ms =
      typeof raw === "string"
        ? Number.parseInt(raw, 10)
        : typeof raw === "number"
          ? raw
          : 0;
    return acc + (Number.isNaN(ms) ? 0 : ms);
  }, 0);
}

export function buildActivityParts(
  steps: ToolStep[],
): { explored: string[]; ran: string[]; duration: string | null; counts: ActivityCounts } {
  const names = collectToolNames(steps);
  const explored: string[] = [];
  const ran: string[] = [];
  for (const name of names) {
    if (EXPLORE_TOOLS.has(name)) {
      explored.push(name);
    } else {
      ran.push(name);
    }
  }
  const duration = formatDurationMs({ duration_ms: totalDurationMs(steps) });
  return { explored, ran, duration, counts: accumulateActivityCounts(steps) };
}

function fillTemplate(template: string, vars: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (_, key) => vars[key] ?? "");
}

type FormatOpts = {
  includeDuration?: boolean;
  preferCounts?: boolean;
};

function formatCountsLine(counts: ActivityCounts, t: (key: string) => string): string | null {
  // Shell-only turns: prefer tool-name recap ("Ran Bash") over "0 files / 0 searches".
  if (
    counts.commands > 0 &&
    counts.files === 0 &&
    counts.searches === 0 &&
    counts.editedFiles === 0
  ) {
    return null;
  }

  const exploreParts: string[] = [];
  if (counts.files > 0) {
    exploreParts.push(
      fillTemplate(t("conversations.activityCountFiles"), {
        files: String(counts.files),
      }),
    );
  }
  if (counts.searches > 0) {
    exploreParts.push(
      fillTemplate(t("conversations.activityCountSearches"), {
        searches: String(counts.searches),
      }),
    );
  }
  if (counts.commands > 0) {
    exploreParts.push(
      fillTemplate(t("conversations.activityCountCommands"), {
        commands: String(counts.commands),
      }),
    );
  }

  const parts: string[] = [];
  if (exploreParts.length > 0) {
    parts.push(exploreParts.join(" · "));
  }
  if (counts.editedFiles > 0) {
    parts.push(
      fillTemplate(t("conversations.activityEditedCounts"), {
        files: String(counts.editedFiles),
        add: String(counts.linesAdded),
        del: String(counts.linesRemoved),
      }),
    );
  }
  if (parts.length === 0) return null;
  return parts.join(" · ");
}

export function formatAgentActivityLine(
  steps: ToolStep[],
  t: (key: string) => string,
  opts: FormatOpts = {},
): string | null {
  if (steps.length === 0) {
    return null;
  }
  const includeDuration = opts.includeDuration !== false;
  const preferCounts = opts.preferCounts !== false;
  const { explored, ran, duration, counts } = buildActivityParts(steps);

  if (preferCounts) {
    const countLine = formatCountsLine(counts, t);
    if (countLine) {
      if (includeDuration && duration) {
        return `${countLine} · ${duration}`;
      }
      return countLine;
    }
  }

  const parts: string[] = [];
  if (explored.length > 0) {
    parts.push(
      fillTemplate(t("conversations.activityExplored"), {
        tools: explored.join(", "),
      }),
    );
  }
  if (ran.length > 0) {
    parts.push(
      fillTemplate(t("conversations.activityRan"), {
        tools: ran.join(", "),
      }),
    );
  }
  if (parts.length === 0) {
    return null;
  }
  let line = parts.join(" · ");
  if (includeDuration && duration) {
    line += ` · ${duration}`;
  }
  return line;
}

export function formatAgentActivityRecap(
  steps: ToolStep[],
  t: (key: string) => string,
): string | null {
  return formatAgentActivityLine(steps, t, { includeDuration: false, preferCounts: true });
}

export function truncateThinkingPreview(text: string, max = 120): string {
  const oneLine = text.replace(/\s+/g, " ").trim();
  if (oneLine.length <= max) {
    return oneLine;
  }
  return `${oneLine.slice(0, max - 1)}…`;
}

export function isStatusMessage(block: TranscriptBlock): boolean {
  return (
    block.meta?.message_role === "status" ||
    (block.block_type === "assistant_message" && Boolean(block.meta?.narration)) ||
    (block.block_type === "system_notice" &&
      block.meta?.source === "intermediate_assistant")
  );
}

export function isFinalAssistantMessage(block: TranscriptBlock): boolean {
  return block.block_type === "assistant_message" && !isStatusMessage(block);
}
