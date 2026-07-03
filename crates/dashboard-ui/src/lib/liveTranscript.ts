import type { TranscriptBlock } from "@/api/types";

export interface ChatStreamEvent {
  session_id: string;
  project_id: string;
  kind: string;
  turn?: number;
  tool_key?: string;
  tool_name?: string;
  text?: string;
  block?: TranscriptBlock;
  payload?: Record<string, unknown>;
  at: string;
}

function liveAssistantId(turn: number): string {
  return `assistant-live:${turn}`;
}

function upsertBlock(blocks: TranscriptBlock[], block: TranscriptBlock): TranscriptBlock[] {
  const idx = blocks.findIndex((b) => b.id === block.id);
  if (idx >= 0) {
    const next = blocks.slice();
    next[idx] = { ...next[idx], ...block };
    return next;
  }
  return [...blocks, block];
}

/** Apply one `chat_event` SSE payload onto transcript blocks. */
export function applyChatStreamEvent(
  blocks: TranscriptBlock[],
  evt: ChatStreamEvent,
): TranscriptBlock[] {
  switch (evt.kind) {
    case "user_message": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "assistant_delta": {
      const turn = evt.turn ?? 1;
      const id = evt.block?.id ?? liveAssistantId(turn);
      const existing = blocks.find((b) => b.id === id);
      const body = evt.block?.body ?? `${existing?.body ?? ""}${evt.text ?? ""}`;
      const block: TranscriptBlock = {
        id,
        block_type: "assistant_message",
        at: evt.at,
        title: evt.block?.title ?? `Assistant (turn ${turn})`,
        body,
        meta: { ...(existing?.meta ?? {}), ...(evt.block?.meta ?? {}), live: true, turn },
        collapsible: false,
        default_collapsed: false,
        event_id: existing?.event_id ?? evt.block?.event_id,
      };
      return upsertBlock(blocks, block);
    }
    case "assistant_done": {
      if (evt.block) {
        return upsertBlock(blocks, {
          ...evt.block,
          meta: { ...(evt.block.meta ?? {}), live: false },
        });
      }
      return blocks;
    }
    case "tool_start":
    case "tool_result": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "llm_start": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "session_error": {
      if (!evt.block) {
        const block: TranscriptBlock = {
          id: `session-error:${evt.at}`,
          block_type: "system_notice",
          at: evt.at,
          title: "Session error",
          body: evt.text ?? "",
          meta: { severity: "error", source: "chat_stream" },
          collapsible: true,
          default_collapsed: false,
        };
        return upsertBlock(blocks, block);
      }
      return upsertBlock(blocks, evt.block);
    }
    case "turn_done":
      return blocks;
    default:
      return blocks;
  }
}

function toolDedupeKey(block: TranscriptBlock): string | null {
  if (block.block_type !== "tool_call" && block.block_type !== "tool_result") {
    return null;
  }
  const toolKey = block.meta?.tool_key;
  if (typeof toolKey !== "string" || !toolKey.trim()) {
    return null;
  }
  return `${block.block_type}:${toolKey.trim()}`;
}

function assistantTurnKey(block: TranscriptBlock): string | null {
  if (block.block_type !== "assistant_message") {
    return null;
  }
  const turn = block.meta?.turn;
  if (turn === undefined || turn === null) {
    return null;
  }
  return `assistant:${String(turn)}`;
}

function mergeBlockContent(prev: TranscriptBlock, incoming: TranscriptBlock): TranscriptBlock {
  const liveFlag = Boolean(incoming.meta?.live);
  if (liveFlag || incoming.body.length >= prev.body.length) {
    return { ...prev, ...incoming };
  }
  return prev;
}

function indexAlternateKeys(
  block: TranscriptBlock,
  id: string,
  toolKeys: Map<string, string>,
  assistantTurns: Map<string, string>,
): void {
  const toolKey = toolDedupeKey(block);
  if (toolKey) {
    toolKeys.set(toolKey, id);
  }
  const assistantKey = assistantTurnKey(block);
  if (assistantKey) {
    assistantTurns.set(assistantKey, id);
  }
}

function resolveExistingId(
  block: TranscriptBlock,
  byId: Map<string, TranscriptBlock>,
  toolKeys: Map<string, string>,
  assistantTurns: Map<string, string>,
): string | null {
  if (byId.has(block.id)) {
    return block.id;
  }
  const toolKey = toolDedupeKey(block);
  if (toolKey && toolKeys.has(toolKey)) {
    return toolKeys.get(toolKey)!;
  }
  const assistantKey = assistantTurnKey(block);
  if (assistantKey && assistantTurns.has(assistantKey)) {
    return assistantTurns.get(assistantKey)!;
  }
  return null;
}

export function mergeTranscriptBlocks(
  snapshot: TranscriptBlock[],
  live: TranscriptBlock[],
): TranscriptBlock[] {
  if (live.length === 0) return snapshot;

  const byId = new Map<string, TranscriptBlock>();
  const toolKeys = new Map<string, string>();
  const assistantTurns = new Map<string, string>();

  for (const block of snapshot) {
    byId.set(block.id, block);
    indexAlternateKeys(block, block.id, toolKeys, assistantTurns);
  }

  for (const block of live) {
    const existingId = resolveExistingId(block, byId, toolKeys, assistantTurns);
    if (existingId) {
      const prev = byId.get(existingId)!;
      byId.set(existingId, mergeBlockContent(prev, { ...block, id: existingId }));
      continue;
    }
    byId.set(block.id, block);
    indexAlternateKeys(block, block.id, toolKeys, assistantTurns);
  }

  const order = [...snapshot.map((b) => b.id)];
  const orderSet = new Set(order);

  for (const block of live) {
    if (byId.has(block.id) && orderSet.has(block.id)) {
      continue;
    }
    const existingId = resolveExistingId(block, byId, toolKeys, assistantTurns);
    if (existingId && orderSet.has(existingId)) {
      continue;
    }
    if (!orderSet.has(block.id)) {
      order.push(block.id);
      orderSet.add(block.id);
    }
  }

  return order.map((id) => byId.get(id)!).filter(Boolean);
}

/** True when SSE live blocks already show assistant/tool/thinking activity. */
export function hasLiveStreamActivity(blocks: TranscriptBlock[]): boolean {
  return blocks.some((block) => {
    if (block.block_type === "assistant_message") {
      return Boolean(block.meta?.live) || block.body.trim().length > 0;
    }
    if (block.block_type === "tool_call" || block.block_type === "tool_result") {
      return true;
    }
    if (block.block_type === "system_notice" && block.meta?.source === "llm_start") {
      return Boolean(block.meta?.live);
    }
    return false;
  });
}
