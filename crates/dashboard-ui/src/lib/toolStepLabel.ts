import type { TranscriptBlock } from "@/api/types";
import { previewLines } from "@/components/ui/CollapsiblePanel";
import type { ToolStep } from "@/lib/transcriptGrouping";
import { toolStepFailed, toolStepRunning } from "@/lib/transcriptGrouping";

export type ToolStepStatus = "running" | "done" | "failed" | "abandoned";

export type ToolStepLabelParts = {
  toolName: string;
  command: string;
  duration: string | null;
  status: ToolStepStatus;
};

function metaRecord(meta: TranscriptBlock["meta"]): Record<string, unknown> {
  return (meta ?? {}) as Record<string, unknown>;
}

const PATH_KEYS = [
  "file_path",
  "path",
  "target_file",
  "notebook_path",
  "command",
  "pattern",
  "glob_pattern",
  "query",
  "description",
] as const;

/** Pull a short path/command from truncated JSON tool input previews. */
function extractFromTruncatedJson(body: string): string {
  for (const key of PATH_KEYS) {
    const re = new RegExp(`"${key}"\\s*:\\s*"((?:\\\\.|[^"\\\\])*)"`, "i");
    const match = body.match(re);
    if (match?.[1]) {
      return match[1].replace(/\\"/g, '"').replace(/\\\\/g, "\\");
    }
  }
  return "";
}

/** Strip " started|finished|failed" status suffixes from tool titles. */
export function stripToolTitleStatus(title: string): string {
  return title.replace(/\s+(started|finished|failed)$/i, "").trim();
}

/** Extract a short command / input preview from a tool block. */
export function extractToolCommand(block: TranscriptBlock | undefined): string {
  if (!block) return "";
  const meta = metaRecord(block.meta);
  for (const key of ["command", "path", "file_path", "query", "description"]) {
    const value = meta[key];
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  const body = block.body?.trim() ?? "";
  if (!body) return "";
  if (body.startsWith("{")) {
    try {
      const parsed = JSON.parse(body) as Record<string, unknown>;
      for (const key of PATH_KEYS) {
        const value = parsed[key];
        if (typeof value === "string" && value.trim()) {
          return value.trim();
        }
      }
      if (Array.isArray(parsed.todos)) {
        return `${parsed.todos.length} todos`;
      }
    } catch {
      const fromTruncated = extractFromTruncatedJson(body);
      if (fromTruncated) return fromTruncated;
    }
  }
  return previewLines(body, 1, 96);
}

export function formatDurationMs(meta: Record<string, unknown> | undefined): string | null {
  if (!meta) return null;
  const raw = meta.duration_ms ?? meta.elapsed_ms;
  const ms =
    typeof raw === "string"
      ? Number.parseInt(raw, 10)
      : typeof raw === "number"
        ? raw
        : Number.NaN;
  if (Number.isNaN(ms) || ms <= 0) return null;
  if (ms >= 10_000) return `${Math.round(ms / 1000)}s`;
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${ms}ms`;
}

function resolveToolDisplayName(
  step: ToolStep,
  formatTitle: (block: TranscriptBlock) => string,
): string {
  const primary = step.result ?? step.call;
  if (!primary) return "Tool";
  const metaName = metaRecord(primary.meta).name;
  if (typeof metaName === "string" && metaName.trim()) {
    return metaName.trim();
  }
  const callName = step.call ? metaRecord(step.call.meta).name : undefined;
  if (typeof callName === "string" && callName.trim()) {
    return callName.trim();
  }
  return stripToolTitleStatus(formatTitle(primary));
}

export function formatToolStepLabelParts(
  step: ToolStep,
  formatTitle: (block: TranscriptBlock) => string,
  opts?: { segmentActive?: boolean },
): ToolStepLabelParts {
  const segmentActive = opts?.segmentActive ?? true;
  const unpaired = toolStepRunning(step);
  const running = unpaired && segmentActive;
  const abandoned = unpaired && !segmentActive;
  const failed = toolStepFailed(step);
  const primary = step.result ?? step.call;
  const toolName = resolveToolDisplayName(step, formatTitle);
  const command =
    extractToolCommand(step.call) ||
    extractToolCommand(step.result) ||
    extractToolCommand(primary);
  const meta = metaRecord(step.result?.meta ?? step.call?.meta);
  return {
    toolName,
    command,
    duration: running ? null : formatDurationMs(meta),
    status: running ? "running" : failed ? "failed" : abandoned ? "abandoned" : "done",
  };
}

export function formatToolStepLabel(
  step: ToolStep,
  formatTitle: (block: TranscriptBlock) => string,
  opts?: { segmentActive?: boolean },
): string {
  const parts = formatToolStepLabelParts(step, formatTitle, opts);
  const segments = [parts.toolName];
  if (parts.command) segments.push(parts.command);
  if (parts.duration) segments.push(parts.duration);
  if (parts.status === "failed" || parts.status === "abandoned") segments.push("✗");
  else if (parts.status === "done") segments.push("✓");
  return segments.join(" · ");
}
