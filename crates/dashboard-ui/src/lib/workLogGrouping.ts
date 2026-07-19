import type { TranscriptBlock } from "@/api/types";
import { isStatusMessage } from "@/lib/agentActivitySummary";
import { isProgressBlock, progressPhase } from "@/lib/progressMeta";
import type { TurnReplyItem, ToolStep } from "@/lib/transcriptGrouping";

export type AgentTurnWorkBundle = {
  progressLines: TranscriptBlock[];
  toolSteps: ToolStep[];
  discoveries: TranscriptBlock[];
  processSnippets: string[];
};

export type AgentTurnRender = {
  work: AgentTurnWorkBundle;
  finalReply?: TranscriptBlock;
  extras: TranscriptBlock[];
};

function isIntermediateAssistant(block: TranscriptBlock): boolean {
  return (
    block.block_type === "assistant_message" &&
    (isStatusMessage(block) || Boolean(block.meta?.narration))
  );
}

function mergeToolSteps(existing: ToolStep[], incoming: ToolStep[]): ToolStep[] {
  const byKey = new Map<string, ToolStep>();
  for (const step of [...existing, ...incoming]) {
    byKey.set(step.key, { ...byKey.get(step.key), ...step, key: step.key });
  }
  return [...byKey.values()];
}

function isIntermediateWorkLine(block: TranscriptBlock): boolean {
  return (
    block.block_type === "system_notice" &&
    block.meta?.source === "intermediate_assistant"
  );
}

function resolveFinalReply(items: TurnReplyItem[]): TranscriptBlock | undefined {
  for (let i = items.length - 1; i >= 0; i -= 1) {
    const item = items[i]!;
    if (item.kind === "tool_cluster") {
      return undefined;
    }
    if (
      item.kind === "block" &&
      item.block.block_type === "assistant_message" &&
      !isStatusMessage(item.block) &&
      item.block.body.trim().length > 0
    ) {
      return item.block;
    }
  }
  return undefined;
}

/** Split a user turn into work evidence + single final reply. */
export function groupTurnForWorkLog(items: TurnReplyItem[]): AgentTurnRender {
  const work: AgentTurnWorkBundle = {
    progressLines: [],
    toolSteps: [],
    discoveries: [],
    processSnippets: [],
  };

  const extras: TranscriptBlock[] = [];
  const finalReply = resolveFinalReply(items);

  for (const item of items) {
    if (item.kind === "tool_cluster") {
      work.toolSteps = mergeToolSteps(work.toolSteps, item.steps);
      work.processSnippets.push(...item.processSnippets);
      continue;
    }

    const block = item.block;
    if (block === finalReply) {
      continue;
    }

    if (block.block_type === "progress_update") {
      if (progressPhase(block) === "discovery") {
        work.discoveries.push(block);
      } else {
        work.progressLines.push(block);
      }
      continue;
    }

    if (isProgressBlock(block) || isIntermediateAssistant(block) || isIntermediateWorkLine(block)) {
      work.progressLines.push(block);
      continue;
    }

    if (
      block.block_type === "assistant_message" &&
      block.body.trim().length > 0
    ) {
      work.progressLines.push(block);
      continue;
    }

    extras.push(block);
  }

  return { work, finalReply, extras };
}

export function latestWorkSummary(
  work: AgentTurnWorkBundle,
  formatBody: (block: TranscriptBlock) => string,
): string | null {
  for (let i = work.progressLines.length - 1; i >= 0; i -= 1) {
    const text = formatBody(work.progressLines[i]!);
    if (text) return text;
  }
  for (let i = work.discoveries.length - 1; i >= 0; i -= 1) {
    const text = formatBody(work.discoveries[i]!);
    if (text) return text;
  }
  const lastSnippet = work.processSnippets[work.processSnippets.length - 1];
  if (lastSnippet?.trim()) return lastSnippet.trim();
  return null;
}
