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
    (block.meta?.source === "intermediate_assistant" ||
      block.meta?.source === "llm_start" ||
      block.meta?.source === "thinking_delta")
  );
}

function isNarrationAssistant(block: TranscriptBlock): boolean {
  return (
    block.block_type === "assistant_message" &&
    (block.meta?.narration === true || block.meta?.message_role === "status")
  );
}

function isProgressAssistant(block: TranscriptBlock): boolean {
  return block.block_type === "progress_update" || isNarrationAssistant(block);
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

function pushThinkingSnippet(processSnippets: string[], snippet: string) {
  const trimmed = snippet.trim();
  if (!trimmed) return;
  const last = processSnippets[processSnippets.length - 1];
  if (last === trimmed) return;
  processSnippets.push(trimmed);
}

/**
 * Merge multiple assistant_message blocks into one final bubble per user turn segment.
 */
export function mergeFinalAssistantBlocks(replies: TranscriptBlock[]): TranscriptBlock[] {
  const out: TranscriptBlock[] = [];
  let buffer: TranscriptBlock[] = [];

  const flush = () => {
    if (buffer.length === 0) return;
    if (buffer.length === 1) {
      out.push(buffer[0]!);
    } else {
      const last = buffer[buffer.length - 1]!;
      const body = buffer
        .map((b) => b.body?.trim() ?? "")
        .filter(Boolean)
        .join("\n\n");
      out.push({
        ...last,
        body,
        meta: {
          ...(last.meta ?? {}),
          live: buffer.some((b) => Boolean(b.meta?.live)),
          merged_assistant: true,
        },
      });
    }
    buffer = [];
  };

  for (const block of replies) {
    if (
      block.block_type === "assistant_message" &&
      !isNarrationAssistant(block)
    ) {
      buffer.push(block);
      continue;
    }
    flush();
    out.push(block);
  }
  flush();
  return out;
}

/**
 * Group tool blocks into per-segment clusters (Cursor/Codex-style interleaving).
 * Agent narration is a user-facing progress update, so it remains a first-class
 * transcript block. Only transport/system notices fold into tool details.
 */
export function groupTurnReplies(replies: TranscriptBlock[]): TurnReplyItem[] {
  const out: TurnReplyItem[] = [];
  let toolBuffer: TranscriptBlock[] = [];
  let processCount = 0;
  const processSnippets: string[] = [];

  const flushTools = () => {
    if (toolBuffer.length === 0 && processCount === 0 && processSnippets.length === 0) {
      return;
    }
    out.push(makeToolCluster(toolBuffer, processCount, [...processSnippets]));
    toolBuffer = [];
    processCount = 0;
    processSnippets.length = 0;
  };

  for (let index = 0; index < replies.length; index += 1) {
    const block = replies[index]!;
    if (isToolBlock(block)) {
      toolBuffer.push(block);
      continue;
    }

    if (isIntermediateAssistantNotice(block)) {
      const snippet = block.body?.trim() ?? "";
      const source = block.meta?.source;
      // Keep user-facing mid-turn narration on the timeline (above tools).
      // Only transport noise (llm_start / empty / thinking_delta) folds into
      // the following tool cluster's thinking strip.
      if (source === "intermediate_assistant" && snippet.length > 0) {
        flushTools();
        out.push({ kind: "block", block });
        continue;
      }
      if (snippet) {
        pushThinkingSnippet(processSnippets, snippet);
      } else {
        processCount += 1;
      }
      continue;
    }

    if (isProgressAssistant(block)) {
      flushTools();
      out.push({ kind: "block", block });
      continue;
    }

    if (block.block_type === "assistant_message") {
      const body = block.body?.trim() ?? "";
      if (body.length === 0 && block.meta?.live !== true) {
        processCount += 1;
        continue;
      }
      // Keep mid-turn assistant narration on the timeline (Claude/Codex-style),
      // even when more tools follow. Do not fold into tool thinking snippets.
      flushTools();
      out.push({ kind: "block", block });
      continue;
    }

    flushTools();
    out.push({ kind: "block", block });
  }

  flushTools();
  return mergeToolClusters(out);
}

function isClusterMergeSeparator(block: TranscriptBlock): boolean {
  if (isProgressAssistant(block)) return false;
  if (isIntermediateAssistantNotice(block)) return false;
  if (block.block_type === "system_notice") return true;
  if (block.block_type === "assistant_message") {
    const body = block.body?.trim() ?? "";
    return body.length === 0 && block.meta?.live !== true;
  }
  return false;
}

function mergeClusterItems(
  left: Extract<TurnReplyItem, { kind: "tool_cluster" }>,
  right: Extract<TurnReplyItem, { kind: "tool_cluster" }>,
  sepSnippets: string[],
  sepCount: number,
): Extract<TurnReplyItem, { kind: "tool_cluster" }> {
  const snippets = [...left.processSnippets];
  for (const snippet of sepSnippets) {
    pushThinkingSnippet(snippets, snippet);
  }
  return {
    kind: "tool_cluster",
    id: left.id,
    steps: [...left.steps, ...right.steps],
    processMessageCount: left.processMessageCount + sepCount + right.processMessageCount,
    processSnippets: [...snippets, ...right.processSnippets],
  };
}

/** Merge tool clusters separated only by narration / status blocks. */
function mergeToolClusters(items: TurnReplyItem[]): TurnReplyItem[] {
  const out: TurnReplyItem[] = [];
  for (let i = 0; i < items.length; i += 1) {
    const item = items[i]!;
    if (item.kind !== "tool_cluster") {
      out.push(item);
      continue;
    }

    let merged: Extract<TurnReplyItem, { kind: "tool_cluster" }> = {
      kind: "tool_cluster",
      id: item.id,
      steps: [...item.steps],
      processMessageCount: item.processMessageCount,
      processSnippets: [...item.processSnippets],
    };

    let j = i + 1;
    while (j < items.length) {
      const sepSnippets: string[] = [];
      let sepCount = 0;
      let k = j;
      while (k < items.length) {
        const mid = items[k]!;
        if (mid.kind === "block" && isClusterMergeSeparator(mid.block)) {
          const snippet = mid.block.body?.trim();
          if (snippet) {
            pushThinkingSnippet(sepSnippets, snippet);
          } else {
            sepCount += 1;
          }
          k += 1;
          continue;
        }
        break;
      }
      const next = items[k];
      if (next?.kind === "tool_cluster" && k > j) {
        merged = mergeClusterItems(merged, next, sepSnippets, sepCount);
        i = k;
        j = k + 1;
        continue;
      }
      break;
    }

    const last = out[out.length - 1];
    if (last?.kind === "tool_cluster") {
      last.steps = [...last.steps, ...merged.steps];
      last.processMessageCount += merged.processMessageCount;
      last.processSnippets = [...last.processSnippets, ...merged.processSnippets];
    } else {
      out.push(merged);
    }
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

export function toolStepRunning(step: ToolStep): boolean {
  return Boolean(step.call) && !step.result;
}

const TOOL_FAILED_TITLE_RE = /\bfailed\b/i;
const TOOL_FINISHED_TITLE_RE = /\bfinished\b/i;

/** 纯文本错误前缀：与输出正文无关的确定性失败信号。 */
const TEXT_FAILURE_MARKERS = [
  "Command failed",
  "Command timed out",
  "File not found",
  "Not a file",
  "Is a directory",
  "Permission denied",
  "rg failed",
  "Serialization error: missing field",
  "path escapes sandbox",
  "skill exited with code",
];

export function toolResultFailed(title: string, body: string): boolean {
  // 1) 服务端生成的 title（"{name} failed/finished"）是最可靠信号。
  if (TOOL_FINISHED_TITLE_RE.test(title)) return false;
  if (TOOL_FAILED_TITLE_RE.test(title)) return true;

  // 2) 结构化 JSON 优先：exit_code / success / error 字段。
  try {
    const parsed = JSON.parse(body);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      const obj = parsed as Record<string, unknown>;
      if (typeof obj.exit_code === "number") return obj.exit_code !== 0;
      if (typeof obj.success === "boolean") return !obj.success;
      if (typeof obj.error === "string" && obj.error) return true;
      // 成功负载字段：FileRead(content) / Glob(filenames) / Grep(matches) / WebSearch(raw)
      if ("content" in obj || "filenames" in obj || "matches" in obj || "raw" in obj) {
        return false;
      }
    }
  } catch {
    // 非 JSON，走纯文本判定。
  }

  // 3) 明确错误前缀，避免把输出正文里的 error/failed 字样误判为失败。
  const trimmed = body.trim();
  if (TEXT_FAILURE_MARKERS.some((m) => trimmed.startsWith(m))) return true;
  if (/^HTTP [45]\d\d\b/.test(trimmed)) return true;
  if (/^Other error:/.test(trimmed)) return true;

  return false;
}

export function toolStepFailed(step: ToolStep): boolean {
  const primary = step.result ?? step.call;
  if (!primary) return false;
  return toolResultFailed(primary.title, primary.body);
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
