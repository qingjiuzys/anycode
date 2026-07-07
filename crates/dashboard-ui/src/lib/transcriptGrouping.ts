import type { TranscriptBlock } from "@/api/types";

export type ToolStep = {
  key: string;
  call?: TranscriptBlock;
  result?: TranscriptBlock;
};

export type TurnReplyItem =
  | { kind: "block"; block: TranscriptBlock }
  | {
      kind: "tool_cluster";
      id: string;
      steps: ToolStep[];
      processMessageCount: number;
      /** Collapsed intermediate assistant snippets (thinking). */
      processSnippets: string[];
    };

function isIntermediateAssistantNotice(block: TranscriptBlock): boolean {
  return (
    block.block_type === "system_notice" &&
    (block.meta?.source === "intermediate_assistant" || block.meta?.source === "llm_start")
  );
}

function isToolBlock(block: TranscriptBlock): boolean {
  return block.block_type === "tool_call" || block.block_type === "tool_result";
}

function buildToolSteps(tools: TranscriptBlock[]): ToolStep[] {
  const byKey = new Map<string, ToolStep>();
  const order: string[] = [];

  for (const tool of tools) {
    const key = toolStepKey(tool) ?? tool.id;
    if (!byKey.has(key)) {
      order.push(key);
      byKey.set(key, { key });
    }
    const slot = byKey.get(key)!;
    if (tool.block_type === "tool_result") {
      slot.result = tool;
    } else {
      slot.call = tool;
    }
  }

  return order.map((key) => byKey.get(key)!);
}

function makeToolCluster(
  tools: TranscriptBlock[],
  processMessageCount: number,
  processSnippets: string[],
): TurnReplyItem {
  return {
    kind: "tool_cluster",
    id: `tools:${tools[0]?.id ?? `process-${processMessageCount}`}`,
    steps: buildToolSteps(tools),
    processMessageCount,
    processSnippets,
  };
}

/**
 * Group tool blocks into per-segment clusters (Cursor/Codex-style interleaving).
 * Contiguous tool + thinking runs become one cluster; assistant text stays in order.
 */
export function groupTurnReplies(replies: TranscriptBlock[]): TurnReplyItem[] {
  const out: TurnReplyItem[] = [];
  let toolBuffer: TranscriptBlock[] = [];
  let processCount = 0;
  const processSnippets: string[] = [];

  const flushTools = () => {
    if (toolBuffer.length === 0 && processCount === 0) {
      return;
    }
    out.push(makeToolCluster(toolBuffer, processCount, [...processSnippets]));
    toolBuffer = [];
    processCount = 0;
    processSnippets.length = 0;
  };

  for (const block of replies) {
    if (isToolBlock(block)) {
      toolBuffer.push(block);
      continue;
    }
    if (isIntermediateAssistantNotice(block)) {
      processCount += 1;
      const snippet = block.body?.trim();
      if (snippet) {
        processSnippets.push(snippet);
      }
      continue;
    }
    flushTools();
    out.push({ kind: "block", block });
  }

  flushTools();
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

export function toolStepRunning(step: ToolStep): boolean {
  return Boolean(step.call) && !step.result;
}

export function toolStepFailed(step: ToolStep): boolean {
  const primary = step.result ?? step.call;
  if (!primary) return false;
  return /failed|error|denied/i.test(`${primary.title} ${primary.body}`);
}

/** Name of the tool currently running in a turn, if any. */
export function findActiveToolInReplies(replies: TranscriptBlock[]): string | null {
  const steps = buildToolSteps(replies.filter(isToolBlock));
  for (let i = steps.length - 1; i >= 0; i -= 1) {
    const step = steps[i]!;
    if (toolStepRunning(step)) {
      return step.call?.title?.replace(/\s+started$/i, "") ?? step.call?.title ?? null;
    }
  }
  return null;
}

export function findActiveToolInExecutionLog(
  lines: { event_type?: string | null; title?: string | null; raw: string }[],
): string | null {
  let lastStart: string | null = null;
  for (const line of lines) {
    if (line.event_type === "tool_call_start") {
      const fromRaw = line.raw.match(/name=([^\s]+)/)?.[1];
      lastStart =
        fromRaw ||
        line.title?.replace(/\s+started$/i, "") ||
        line.title ||
        null;
    }
    if (line.event_type === "tool_call_end") {
      lastStart = null;
    }
  }
  return lastStart;
}
