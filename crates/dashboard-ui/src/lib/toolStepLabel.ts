import type { TranscriptBlock } from "@/api/types";
import { previewLines } from "@/components/ui/CollapsiblePanel";
import type { ToolStep } from "@/lib/transcriptGrouping";
import { toolStepFailed, toolStepRunning } from "@/lib/transcriptGrouping";

export type ToolStepStatus = "running" | "done" | "failed";

export type ToolStepLabelParts = {
  toolName: string;
  command: string;
  duration: string | null;
  status: ToolStepStatus;
};

function metaRecord(meta: TranscriptBlock["meta"]): Record<string, unknown> {
  return (meta ?? {}) as Record<string, unknown>;
}

/** Extract a short command / input preview from a tool block. */
export function extractToolCommand(block: TranscriptBlock | undefined): string {
  if (!block) return "";
  const meta = metaRecord(block.meta);
  for (const key of ["command", "path", "query", "description"]) {
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
      for (const key of ["command", "path", "description", "pattern", "glob_pattern"]) {
        const value = parsed[key];
        if (typeof value === "string" && value.trim()) {
          return value.trim();
        }
      }
      if (Array.isArray(parsed.todos)) {
        return `${parsed.todos.length} todos`;
      }
      if (typeof parsed.file_path === "string" && parsed.file_path.trim()) {
        return parsed.file_path.trim();
      }
    } catch {
      /* fall through */
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

export function formatToolStepLabelParts(
  step: ToolStep,
  formatTitle: (block: TranscriptBlock) => string,
): ToolStepLabelParts {
  const running = toolStepRunning(step);
  const failed = toolStepFailed(step);
  const primary = step.result ?? step.call;
  const toolName = primary ? formatTitle(primary) : "Tool";
  const command =
    extractToolCommand(step.call) ||
    extractToolCommand(step.result) ||
    extractToolCommand(primary);
  const meta = metaRecord(step.result?.meta ?? step.call?.meta);
  return {
    toolName,
    command,
    duration: running ? null : formatDurationMs(meta),
    status: running ? "running" : failed ? "failed" : "done",
  };
}

export function formatToolStepLabel(
  step: ToolStep,
  formatTitle: (block: TranscriptBlock) => string,
): string {
  const parts = formatToolStepLabelParts(step, formatTitle);
  const segments = [parts.toolName];
  if (parts.command) segments.push(parts.command);
  if (parts.duration) segments.push(parts.duration);
  if (parts.status === "failed") segments.push("✗");
  else if (parts.status === "done") segments.push("✓");
  return segments.join(" · ");
}
