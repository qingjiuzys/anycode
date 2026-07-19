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

function turnDoneNoticeBlock(evt: ChatStreamEvent): TranscriptBlock | null {
  const status =
    (typeof evt.payload?.status === "string" ? evt.payload.status : null) ??
    evt.text ??
    "completed";
  if (status === "completed") return null;
  const userTurnId = evt.conversation_turn_id ?? 1;
  const copy =
    status === "max_turns"
      ? {
          title: "已达模型轮次上限",
          body: "任务在未完全完成时中断；下方为部分进度总结。可在 config.json 的 runtime.max_agent_turns 提高上限。",
        }
      : status === "max_tools"
        ? {
            title: "已达工具调用上限",
            body: "任务因 max_tool_calls 限制中断；下方为部分进度总结。可在 config.json 的 runtime.max_tool_calls 提高上限。",
          }
        : status === "budget"
          ? {
              title: "已达预算上限",
              body: "任务因 token/费用预算限制中断；下方为部分进度总结。",
            }
          : status === "refusal_no_tool"
            ? {
                title: "模型未调用工具",
                body: "首轮未触发工具调用即结束；可换模型或调整 prompt 后重试。",
              }
            : status === "cancelled"
              ? {
                  title: "任务已取消",
                  body: "本轮执行被用户或系统中止；下方为已完成部分的总结（如有）。",
                }
              : { title: "任务结束", body: status };
  const severity =
    status === "max_turns" ||
    status === "max_tools" ||
    status === "budget" ||
    status === "refusal_no_tool"
      ? "warning"
      : "info";
  return {
    id: `turn-done:u${userTurnId}:${status}`,
    block_type: "system_notice",
    at: evt.at,
    title: copy.title,
    body: copy.body,
    meta: {
      source: "turn_done",
      status,
      severity,
      user_turn_id: String(userTurnId),
    },
    collapsible: false,
    default_collapsed: false,
  };
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
          ...(evt.block?.meta?.narration === true
            ? { narration: true, message_role: "status" as const }
            : {}),
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
    case "progress_update": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, {
        ...evt.block,
        meta: { ...(evt.block.meta ?? {}), live: evt.block.meta?.live ?? true },
      });
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
    case "thinking_delta": {
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
    case "approval_request":
    case "approval_resolved": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "question_request":
    case "question_resolved": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "message_queued": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "message_dequeued": {
      const queueId = evt.payload?.queue_id;
      if (typeof queueId !== "string" || !queueId) return blocks;
      return blocks.filter(
        (b) => b.meta?.queue_id !== queueId && b.id !== `queue:${queueId}`,
      );
    }
    case "turn_phase": {
      if (!evt.block) return blocks;
      return upsertBlock(blocks, evt.block);
    }
    case "turn_done": {
      const notice = turnDoneNoticeBlock(evt);
      return notice ? upsertBlock(blocks, notice) : blocks;
    }
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

/** Merge REST snapshot with in-memory SSE: baseline through last user message + live tail overlay. */
export function resolveCanonicalTranscriptBlocks(
  snapshot: TranscriptBlock[],
  liveEvents: ChatStreamEvent[],
  _snapshotMaxSeq = 0,
  streaming = false,
): TranscriptBlock[] {
  if (!streaming || liveEvents.length === 0) {
    return snapshot;
  }
  const liveBlocks = blocksFromCanonicalEvents(liveEvents);
  if (liveBlocks.length === 0) {
    return snapshot;
  }
  const lastUserIdx = lastUserMessageIndex(snapshot);
  const baseline = lastUserIdx >= 0 ? snapshot.slice(0, lastUserIdx + 1) : [];
  return mergeTranscriptBlocks(baseline, liveBlocks);
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
