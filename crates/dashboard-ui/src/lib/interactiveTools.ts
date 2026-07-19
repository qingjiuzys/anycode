import type { ToolStep } from "@/lib/transcriptGrouping";

/** Tools whose UX is driven by inline inbox UI, not tool-strip chrome. */
export const INTERACTIVE_TOOL_NAMES = new Set([
  "AskUserQuestion",
  "AskUser",
]);

export function toolNameFromStep(step: ToolStep): string {
  const primary = step.result ?? step.call;
  if (!primary) return "";
  const meta = primary.meta as Record<string, unknown> | undefined;
  const metaName = meta?.name;
  if (typeof metaName === "string" && metaName.trim()) {
    return metaName.trim();
  }
  return primary.title.replace(/\s+(started|finished|failed)$/i, "").trim();
}

export function isInteractiveToolName(name: string): boolean {
  return INTERACTIVE_TOOL_NAMES.has(name.trim());
}

export function isInteractiveToolStep(step: ToolStep): boolean {
  return isInteractiveToolName(toolNameFromStep(step));
}

export function isInteractiveToolCluster(steps: ToolStep[]): boolean {
  if (steps.length === 0) return false;
  return steps.every(isInteractiveToolStep);
}

function parseJsonBody(body: string): Record<string, unknown> | null {
  const trimmed = body.trim();
  if (!trimmed.startsWith("{")) return null;
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

/** Short label for settled interactive tool history (e.g. question header). */
export function interactiveStepHistoryLabel(step: ToolStep): string | null {
  const primary = step.call ?? step.result;
  if (!primary) return null;
  const name = toolNameFromStep(step);
  if (!isInteractiveToolName(name)) return null;

  const meta = (primary.meta ?? {}) as Record<string, unknown>;
  for (const key of ["header", "question"]) {
    const value = meta[key];
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }

  const fromCall = parseJsonBody(step.call?.body ?? "");
  if (fromCall) {
    for (const key of ["header", "question"]) {
      const value = fromCall[key];
      if (typeof value === "string" && value.trim()) {
        return value.trim();
      }
    }
  }

  return name;
}

export function shouldHideInteractiveCluster(opts: {
  isLast: boolean;
  isRunning: boolean;
  steps: ToolStep[];
  pendingQuestionsCount: number;
  pendingApprovalsCount: number;
}): boolean {
  if (!opts.isLast || !opts.isRunning) return false;
  if (opts.pendingQuestionsCount > 0 || opts.pendingApprovalsCount > 0) {
    return isInteractiveToolCluster(opts.steps);
  }
  return isInteractiveToolCluster(opts.steps);
}
