import type { TranscriptBlock } from "@/api/types";

export interface ChatStreamEvent {
  session_id: string;
  project_id: string;
  kind: string;
  turn?: number;
  conversation_turn_id?: number;
  seq?: number;
  event_id?: string;
  tool_key?: string;
  tool_name?: string;
  text?: string;
  block?: TranscriptBlock;
  payload?: Record<string, unknown>;
  at: string;
}

function liveAssistantId(turn: number, userTurnId?: string | number | null): string {
  if (userTurnId !== undefined && userTurnId !== null && String(userTurnId).length > 0) {
    return `assistant-live:u${userTurnId}:${turn}`;
  }
  return `assistant-live:${turn}`;
}

function userTurnIdFromEvent(evt: ChatStreamEvent): string | undefined {
  const fromPayload = evt.payload?.user_turn_id;
  if (fromPayload !== undefined && fromPayload !== null) {
    return String(fromPayload);
  }
  const fromMeta = evt.block?.meta?.user_turn_id;
  if (fromMeta !== undefined && fromMeta !== null) {
    return String(fromMeta);
  }
  return undefined;
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
      const userTurnId = userTurnIdFromEvent(evt);
      const id = evt.block?.id ?? liveAssistantId(turn, userTurnId);
      const existing = blocks.find((b) => b.id === id);
      const body = evt.block?.body ?? `${existing?.body ?? ""}${evt.text ?? ""}`;
      const block: TranscriptBlock = {
        id,
        block_type: "assistant_message",
        at: evt.at,
        title: evt.block?.title ?? `Assistant (turn ${turn})`,
        body,
        meta: {
          ...(existing?.meta ?? {}),
          ...(evt.block?.meta ?? {}),
          live: true,
          turn,
          ...(userTurnId ? { user_turn_id: userTurnId } : {}),
        },
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
    case "tool_result":
    case "tool_progress": {
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
  const userTurnId = block.meta?.user_turn_id;
  if (userTurnId !== undefined && userTurnId !== null && String(userTurnId).length > 0) {
    return `assistant:u${String(userTurnId)}:${String(turn)}`;
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

function lastUserMessageIndex(blocks: TranscriptBlock[]): number {
  let last = -1;
  for (let i = 0; i < blocks.length; i++) {
    if (blocks[i]?.block_type === "user_message") {
      last = i;
    }
  }
  return last;
}

function isActiveTailBlock(snapshot: TranscriptBlock[], index: number): boolean {
  return index > lastUserMessageIndex(snapshot);
}

export function mergeTranscriptBlocks(
  snapshot: TranscriptBlock[],
  live: TranscriptBlock[],
): TranscriptBlock[] {
  if (live.length === 0) return snapshot;

  const byId = new Map<string, TranscriptBlock>();
  const toolKeys = new Map<string, string>();
  const assistantTurns = new Map<string, string>();
  const blockIndex = new Map<string, number>();

  for (let i = 0; i < snapshot.length; i++) {
    const block = snapshot[i]!;
    byId.set(block.id, block);
    blockIndex.set(block.id, i);
    if (isActiveTailBlock(snapshot, i)) {
      indexAlternateKeys(block, block.id, toolKeys, assistantTurns);
    }
  }

  const order = [...snapshot.map((b) => b.id)];
  const orderSet = new Set(order);
  const activeFrom = lastUserMessageIndex(snapshot) + 1;

  const canMergeInto = (existingId: string): boolean => {
    const idx = blockIndex.get(existingId);
    return idx !== undefined && idx >= activeFrom;
  };

  for (const block of live) {
    const existingId = resolveExistingId(block, byId, toolKeys, assistantTurns);
    if (existingId && canMergeInto(existingId)) {
      const prev = byId.get(existingId)!;
      byId.set(existingId, mergeBlockContent(prev, { ...block, id: existingId }));
      continue;
    }
    byId.set(block.id, block);
    indexAlternateKeys(block, block.id, toolKeys, assistantTurns);
  }

  for (const block of live) {
    if (byId.has(block.id) && orderSet.has(block.id)) {
      continue;
    }
    const existingId = resolveExistingId(block, byId, toolKeys, assistantTurns);
    if (existingId && orderSet.has(existingId) && canMergeInto(existingId)) {
      continue;
    }
    if (!orderSet.has(block.id)) {
      order.push(block.id);
      orderSet.add(block.id);
    }
  }

  return order.map((id) => byId.get(id)!).filter(Boolean);
}

/** Append-only canonical merge: REST snapshot + SSE events with seq > snapshotMaxSeq. */
export function resolveCanonicalTranscriptBlocks(
  snapshot: TranscriptBlock[],
  liveEvents: ChatStreamEvent[],
  snapshotMaxSeq = 0,
  streaming = false,
): TranscriptBlock[] {
  if (!streaming || liveEvents.length === 0) {
    return snapshot;
  }
  const pending = liveEvents
    .filter((evt) => (evt.seq ?? 0) > snapshotMaxSeq)
    .sort((a, b) => (a.seq ?? 0) - (b.seq ?? 0));
  if (pending.length === 0) {
    return snapshot;
  }
  let blocks = [...snapshot];
  for (const evt of pending) {
    blocks = applyChatStreamEvent(blocks, evt);
  }
  return blocks;
}

/** @deprecated Prefer resolveCanonicalTranscriptBlocks with liveEvents + max_seq. */
export function resolveTranscriptBlocks(
  snapshot: TranscriptBlock[],
  live: TranscriptBlock[],
  streaming: boolean,
): TranscriptBlock[] {
  if (!streaming || live.length === 0) {
    return snapshot;
  }
  const lastUserIdx = lastUserMessageIndex(snapshot);
  const baseline = lastUserIdx >= 0 ? snapshot.slice(0, lastUserIdx + 1) : [];
  return mergeTranscriptBlocks(baseline, live);
}

/** Build transcript blocks from canonical SSE events ordered by seq. */
export function blocksFromCanonicalEvents(events: ChatStreamEvent[]): TranscriptBlock[] {
  const sorted = [...events].sort((a, b) => (a.seq ?? 0) - (b.seq ?? 0));
  let blocks: TranscriptBlock[] = [];
  for (const evt of sorted) {
    blocks = applyChatStreamEvent(blocks, evt);
  }
  return blocks;
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

/** Running turn has visible SSE or execution-log activity (not idle waiting). */
export function hasTurnStreamActivity(
  liveBlocks: TranscriptBlock[],
  activeToolName: string | null | undefined,
  lastTurnReplies: TranscriptBlock[] = [],
): boolean {
  if (hasLiveStreamActivity(liveBlocks)) {
    return true;
  }
  if (activeToolName) {
    return true;
  }
  for (const block of lastTurnReplies) {
    if (block.block_type === "tool_call" && !lastTurnReplies.some(
      (other) =>
        other.block_type === "tool_result" &&
        other.meta?.tool_key === block.meta?.tool_key,
    )) {
      return true;
    }
    if (block.block_type === "assistant_message" && block.body.trim().length > 0) {
      return true;
    }
    if (
      block.block_type === "system_notice" &&
      (block.meta?.source === "llm_start" || block.meta?.source === "intermediate_assistant")
    ) {
      return true;
    }
  }
  return false;
}
