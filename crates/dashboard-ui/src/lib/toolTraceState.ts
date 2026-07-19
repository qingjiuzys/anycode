import {
  toolStepRunning,
  type ToolStep,
  type TurnReplyItem,
} from "@/lib/transcriptGrouping";

/** Whether the tool trace should use the live/streaming layout. */
export function toolTraceStreaming(
  steps: ToolStep[],
  processMessageCount: number,
  segmentActive: boolean,
): boolean {
  // Session/segment already settled → never spin, even with unpaired tool_call.
  if (!segmentActive) {
    return false;
  }
  if (steps.some(toolStepRunning)) {
    return true;
  }
  return steps.length === 0 && processMessageCount > 0;
}

/** Show the "thinking…" header only before the first tool step lands. */
export function toolTraceShowThinkingHeader(
  steps: ToolStep[],
  processMessageCount: number,
  segmentActive: boolean,
): boolean {
  return (
    toolTraceStreaming(steps, processMessageCount, segmentActive) &&
    steps.length === 0 &&
    processMessageCount > 0
  );
}

/** True when a later assistant reply or tool round has already started. */
export function toolClusterSegmentSettled(
  replyItems: TurnReplyItem[],
  clusterIndex: number,
): boolean {
  for (let i = clusterIndex + 1; i < replyItems.length; i++) {
    const row = replyItems[i]!;
    if (row.kind === "tool_cluster") {
      return true;
    }
    if (row.kind === "block") {
      const block = row.block;
      if (
        block.block_type === "progress_update" ||
        (block.block_type === "system_notice" &&
          block.meta?.source === "intermediate_assistant" &&
          (block.body?.trim()?.length ?? 0) > 0)
      ) {
        return true;
      }
      if (block.block_type === "assistant_message") {
        const body = block.body?.trim() ?? "";
        if (body.length > 0) {
          return true;
        }
      }
    }
  }
  return false;
}

/** Whether this cluster should still receive the session-running flag. */
export function toolClusterSegmentActive(
  steps: ToolStep[],
  sessionRunning: boolean,
  isLastClusterOnLastTurn: boolean,
  settled: boolean,
): boolean {
  if (!sessionRunning || !isLastClusterOnLastTurn || settled) {
    return false;
  }
  if (steps.some(toolStepRunning)) {
    return true;
  }
  if (steps.length > 0) {
    return false;
  }
  return true;
}

function isNarrationLikeBlock(block: {
  block_type: string;
  meta?: Record<string, unknown> | null;
  body?: string;
}): boolean {
  if (block.block_type === "progress_update") return true;
  if (
    block.block_type === "system_notice" &&
    (block.meta?.source === "intermediate_assistant" ||
      block.meta?.source === "thinking_delta" ||
      block.meta?.source === "llm_start")
  ) {
    return true;
  }
  if (
    block.block_type === "assistant_message" &&
    (block.meta?.narration === true || block.meta?.message_role === "status")
  ) {
    return true;
  }
  return false;
}

/**
 * Index of the single timeline segment that should stay expanded (accordion).
 * While the turn is running: always the tip (last) segment.
 * When settled: -1 (tools stay one-line; final assistant opened separately).
 */
export function resolveActiveReplySegment(
  replyItems: TurnReplyItem[],
  opts: { isLast: boolean; isRunning: boolean },
): number {
  if (!opts.isLast || !opts.isRunning || replyItems.length === 0) {
    return -1;
  }

  // Codex/Cursor-style: the newest timeline item is always the open one.
  // Prior segments collapse as soon as a newer block/cluster arrives.
  return replyItems.length - 1;
}

/** Last non-narration assistant_message index (final deliverable bubble). */
export function resolveFinalAssistantIndex(replyItems: TurnReplyItem[]): number {
  for (let i = replyItems.length - 1; i >= 0; i--) {
    const item = replyItems[i]!;
    if (item.kind !== "block") continue;
    const block = item.block;
    if (block.block_type !== "assistant_message") continue;
    if (isNarrationLikeBlock(block)) continue;
    if ((block.body?.trim()?.length ?? 0) > 0 || block.meta?.live === true) {
      return i;
    }
  }
  return -1;
}
