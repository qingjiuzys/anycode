import type { TranscriptBlock } from "@/api/types";

export type TurnReplyItem =
  | { kind: "block"; block: TranscriptBlock }
  | {
      kind: "tool_group";
      id: string;
      tools: TranscriptBlock[];
      processMessageCount: number;
    };

function isIntermediateAssistantNotice(block: TranscriptBlock): boolean {
  return (
    block.block_type === "system_notice" &&
    block.meta?.source === "intermediate_assistant"
  );
}

function isToolBlock(block: TranscriptBlock): boolean {
  return block.block_type === "tool_call" || block.block_type === "tool_result";
}

/** Merge all tool blocks in a user turn into one execution strip; fold intermediate replies into counts. */
export function groupTurnReplies(replies: TranscriptBlock[]): TurnReplyItem[] {
  const tools: TranscriptBlock[] = [];
  let processMessageCount = 0;
  let firstToolIndex = -1;

  for (let i = 0; i < replies.length; i += 1) {
    const block = replies[i]!;
    if (isToolBlock(block)) {
      if (firstToolIndex < 0) {
        firstToolIndex = i;
      }
      tools.push(block);
      continue;
    }
    if (isIntermediateAssistantNotice(block)) {
      processMessageCount += 1;
    }
  }

  if (tools.length === 0 && processMessageCount === 0) {
    return replies.map((block) => ({ kind: "block" as const, block }));
  }

  const toolGroup: TurnReplyItem = {
    kind: "tool_group",
    id: `tools:${tools[0]?.id ?? `process-${processMessageCount}`}`,
    tools,
    processMessageCount,
  };

  const out: TurnReplyItem[] = [];
  let groupInserted = false;

  for (let i = 0; i < replies.length; i += 1) {
    const block = replies[i]!;
    if (isToolBlock(block) || isIntermediateAssistantNotice(block)) {
      if (!groupInserted && (firstToolIndex < 0 || i === firstToolIndex)) {
        out.push(toolGroup);
        groupInserted = true;
      }
      continue;
    }
    out.push({ kind: "block", block });
  }

  if (!groupInserted) {
    out.unshift(toolGroup);
  }

  return out;
}

/** Count logical tool invocations (paired start/end), not raw transcript blocks. */
export function countLogicalToolSteps(tools: TranscriptBlock[]): number {
  const keys = new Set<string>();
  for (const tool of tools) {
    const key = toolStepKey(tool);
    if (key) {
      keys.add(key);
    }
  }
  if (keys.size > 0) {
    return keys.size;
  }
  const calls = tools.filter((t) => t.block_type === "tool_call").length;
  if (calls > 0) {
    return calls;
  }
  return Math.max(1, Math.ceil(tools.length / 2));
}

export function toolStepKey(tool: TranscriptBlock): string | null {
  const meta = tool.meta;
  if (!meta) {
    return null;
  }
  const toolKey = meta.tool_key;
  if (typeof toolKey === "string" && toolKey.trim()) {
    return toolKey.trim();
  }
  const turn = meta.turn;
  const idx = meta.idx;
  if (typeof turn === "string" && typeof idx === "string" && turn && idx) {
    return `${turn}:${idx}`;
  }
  return tool.event_id ?? tool.id;
}
