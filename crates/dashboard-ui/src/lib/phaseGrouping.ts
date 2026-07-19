import type { TranscriptBlock } from "@/api/types";
import {
  isProgressBlock,
  progressPhase,
  type AgentPhaseKind,
} from "@/lib/progressMeta";
import type { TurnReplyItem } from "@/lib/transcriptGrouping";
import { isFinalAssistantMessage } from "@/lib/agentActivitySummary";

export type AgentPhaseSegment = {
  id: string;
  phase: AgentPhaseKind;
  progressBlock?: TranscriptBlock;
  toolCluster?: Extract<TurnReplyItem, { kind: "tool_cluster" }>;
  deliverBlock?: TranscriptBlock;
  extras: TranscriptBlock[];
};

function isDeliverBlock(block: TranscriptBlock): boolean {
  if (block.block_type === "progress_update") {
    return progressPhase(block) === "deliver";
  }
  return isFinalAssistantMessage(block);
}

/** Hide narration assistant_message when a structured progress_update exists for the same model turn. */
export function dedupeNarrationWithProgress(items: TurnReplyItem[]): TurnReplyItem[] {
  const progressTurns = new Set<number>();
  for (const item of items) {
    if (item.kind !== "block") continue;
    if (item.block.block_type !== "progress_update") continue;
    const turn = item.block.meta?.turn;
    if (typeof turn === "number") {
      progressTurns.add(turn);
    } else if (typeof turn === "string") {
      const n = Number.parseInt(turn, 10);
      if (!Number.isNaN(n)) progressTurns.add(n);
    }
  }
  if (progressTurns.size === 0) return items;

  return items.filter((item) => {
    if (item.kind !== "block") return true;
    const block = item.block;
    if (block.block_type !== "assistant_message") return true;
    if (block.meta?.narration !== true && block.meta?.message_role !== "status") {
      return true;
    }
    const turn = block.meta?.turn;
    const turnNum =
      typeof turn === "number"
        ? turn
        : typeof turn === "string"
          ? Number.parseInt(turn, 10)
          : Number.NaN;
    return Number.isNaN(turnNum) || !progressTurns.has(turnNum);
  });
}

export function groupTurnRepliesByPhase(items: TurnReplyItem[]): AgentPhaseSegment[] {
  const segments: AgentPhaseSegment[] = [];
  let pendingTools: Extract<TurnReplyItem, { kind: "tool_cluster" }> | null = null;
  let fallbackPhase: AgentPhaseKind = "intent";

  const flushPendingTools = () => {
    if (!pendingTools) return;
    const last = segments[segments.length - 1];
    if (last && !last.toolCluster) {
      last.toolCluster = pendingTools;
    } else {
      segments.push({
        id: pendingTools.id,
        phase: "execute",
        toolCluster: pendingTools,
        extras: [],
      });
    }
    pendingTools = null;
  };

  for (const item of items) {
    if (item.kind === "tool_cluster") {
      pendingTools = item;
      continue;
    }

    const block = item.block;
    if (isProgressBlock(block)) {
      flushPendingTools();
      const phase = progressPhase(block);
      segments.push({
        id: block.id,
        phase,
        progressBlock: block,
        extras: [],
      });
      fallbackPhase = phase === "intent" ? "execute" : phase;
      continue;
    }

    if (isDeliverBlock(block)) {
      flushPendingTools();
      const last = segments[segments.length - 1];
      if (
        last?.phase === "deliver" &&
        last.progressBlock &&
        !last.deliverBlock &&
        block.block_type === "assistant_message"
      ) {
        last.deliverBlock = block;
        continue;
      }
      segments.push({
        id: block.id,
        phase: "deliver",
        deliverBlock: block,
        extras: [],
      });
      continue;
    }

    flushPendingTools();
    const last = segments[segments.length - 1];
    if (last) {
      last.extras.push(block);
    } else {
      segments.push({
        id: `extra:${block.id}`,
        phase: fallbackPhase,
        extras: [block],
      });
    }
  }

  flushPendingTools();
  return segments;
}

export function latestProgressSummary(
  segments: AgentPhaseSegment[],
  formatBody: (block: TranscriptBlock) => string,
): string | null {
  for (let i = segments.length - 1; i >= 0; i -= 1) {
    const seg = segments[i]!;
    if (seg.progressBlock) {
      const text = formatBody(seg.progressBlock);
      if (text) return text;
    }
  }
  return null;
}

export const MAX_VISIBLE_PHASE_HISTORY = 3;

export function phaseVisibilityPlan(
  segments: AgentPhaseSegment[],
  isLiveTurn: boolean,
): { visible: AgentPhaseSegment[]; archived: AgentPhaseSegment[] } {
  if (!isLiveTurn) {
    return { visible: segments, archived: [] };
  }

  const nonDeliver = segments.filter((s) => s.phase !== "deliver");
  const deliver = segments.filter((s) => s.phase === "deliver");

  if (nonDeliver.length <= MAX_VISIBLE_PHASE_HISTORY) {
    return { visible: segments, archived: [] };
  }

  const splitAt = nonDeliver.length - MAX_VISIBLE_PHASE_HISTORY;
  const archived = nonDeliver.slice(0, splitAt);
  const visibleNonDeliver = nonDeliver.slice(splitAt);
  return {
    visible: [...visibleNonDeliver, ...deliver],
    archived,
  };
}
