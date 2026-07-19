import type { Edge, Node } from "reactflow";
import type { TranscriptBlock } from "@/api/types";
import {
  isInteractiveToolName,
  toolNameFromStep,
} from "@/lib/interactiveTools";
import { extractToolCommand, formatDurationMs } from "@/lib/toolStepLabel";
import {
  toolStepFailed,
  toolStepKey,
  toolStepRunning,
  type ToolStep,
} from "@/lib/transcriptGrouping";

export type TraceNodeKind =
  | "user"
  | "assistant"
  | "tool"
  | "decision"
  | "branch";

export type TraceNodeStatus = "neutral" | "ok" | "failed" | "running" | "chosen" | "skipped";

export type TraceGraphNodeData = {
  kind: TraceNodeKind;
  label: string;
  subtitle?: string;
  /** Primary transcript block for selection / detail. */
  block?: TranscriptBlock;
  status?: TraceNodeStatus;
  failed?: boolean;
  running?: boolean;
  live?: boolean;
  chosen?: boolean;
  /** Stable key for i18n on binary approval branches. */
  branchKey?: "allow" | "deny";
};

export type ExecutionTraceGraphModel = {
  nodes: Node<TraceGraphNodeData>[];
  edges: Edge[];
};

const NODE_GAP_Y = 86;
const NODE_X = 24;
const BRANCH_SPREAD_X = 118;

export const APPROVAL_ALLOW_LABEL = "Allow";
export const APPROVAL_DENY_LABEL = "Deny";

type TurnBucket = {
  id: string;
  user: TranscriptBlock;
  replies: TranscriptBlock[];
};

type LayoutCursor = {
  y: number;
};

type ForkSpec = {
  decisionId: string;
  label: string;
  subtitle?: string;
  block?: TranscriptBlock;
  options: string[];
  selected: string[];
  running?: boolean;
  /** Parallel to options — optional branch metadata. */
  branchKeys?: Array<"allow" | "deny" | undefined>;
};

function blocksToTurns(blocks: TranscriptBlock[]): TurnBucket[] {
  const turns: TurnBucket[] = [];
  let current: TurnBucket | null = null;

  for (const block of blocks) {
    if (block.block_type === "user_message") {
      if (current) turns.push(current);
      current = { id: block.id, user: block, replies: [] };
      continue;
    }
    if (!current) continue;
    if (
      block.block_type === "assistant_message" ||
      block.block_type === "session_error" ||
      block.block_type === "tool_call" ||
      block.block_type === "tool_result" ||
      block.block_type === "system_notice" ||
      block.block_type === "progress_update" ||
      block.block_type === "question_request" ||
      block.block_type === "approval_request"
    ) {
      current.replies.push(block);
    }
  }
  if (current) turns.push(current);
  return turns;
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

function previewText(text: string, max = 72): string {
  const oneLine = text.replace(/\s+/g, " ").trim();
  if (oneLine.length <= max) return oneLine;
  return `${oneLine.slice(0, max - 1)}…`;
}

function toolDisplayName(block: TranscriptBlock | undefined): string {
  if (!block) return "Tool";
  const metaName = typeof block.meta?.name === "string" ? block.meta.name.trim() : "";
  if (metaName) return metaName;
  return block.title.replace(/\s+(started|finished|failed|开始|完成|失败)$/i, "").trim() || "Tool";
}

function parseJsonBody(body: string | undefined): Record<string, unknown> | null {
  const trimmed = body?.trim() ?? "";
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

function metaRecord(block: TranscriptBlock | undefined): Record<string, unknown> {
  return (block?.meta ?? {}) as Record<string, unknown>;
}

function optionLabelsFromValue(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((o) => {
      if (typeof o === "string") return o.trim();
      if (!o || typeof o !== "object") return "";
      const label = (o as Record<string, unknown>).label;
      return typeof label === "string" ? label.trim() : "";
    })
    .filter(Boolean);
}

function optionLabelsFromStep(step: ToolStep): string[] {
  const fromMeta = optionLabelsFromValue(metaRecord(step.call).options);
  if (fromMeta.length > 0) return fromMeta;
  return optionLabelsFromValue(parseJsonBody(step.call?.body)?.options);
}

function selectedLabelsFromStep(step: ToolStep): string[] {
  const body = parseJsonBody(step.result?.body);
  const selected = body?.selected;
  if (Array.isArray(selected)) {
    return selected
      .map((s) => (typeof s === "string" ? s.trim() : ""))
      .filter(Boolean);
  }
  const meta = metaRecord(step.result);
  const metaSelected = meta.selected_labels ?? meta.selected;
  if (Array.isArray(metaSelected)) {
    return metaSelected
      .map((s) => (typeof s === "string" ? s.trim() : ""))
      .filter(Boolean);
  }
  return [];
}

function decisionTitle(step: ToolStep): string {
  const meta = metaRecord(step.call);
  for (const key of ["header", "question"]) {
    const v = meta[key];
    if (typeof v === "string" && v.trim()) return v.trim();
  }
  const body = parseJsonBody(step.call?.body);
  for (const key of ["header", "question"]) {
    const v = body?.[key];
    if (typeof v === "string" && v.trim()) return v.trim();
  }
  return toolNameFromStep(step) || "Question";
}

function turnHasAskUserTool(replies: TranscriptBlock[]): boolean {
  return replies.some((b) => {
    if (!isToolBlock(b)) return false;
    const name =
      (typeof b.meta?.name === "string" && b.meta.name.trim()) ||
      b.title.replace(/\s+(started|finished|failed)$/i, "").trim();
    return isInteractiveToolName(name);
  });
}

function questionRequestsInTurn(replies: TranscriptBlock[]): TranscriptBlock[] {
  return replies.filter((b) => b.block_type === "question_request");
}

/** Fill missing AskUserQuestion options from a pending question_request block. */
function mergeOptionsFromQuestionRequests(
  step: ToolStep,
  replies: TranscriptBlock[],
): string[] {
  const fromStep = optionLabelsFromStep(step);
  if (fromStep.length > 0) return fromStep;
  for (const qr of questionRequestsInTurn(replies)) {
    const labels = optionLabelsFromValue(metaRecord(qr).options);
    if (labels.length > 0) return labels;
  }
  return [];
}

function approvalIdOf(block: TranscriptBlock): string | null {
  const id = metaRecord(block).approval_id;
  return typeof id === "string" && id.trim() ? id.trim() : null;
}

function findApprovalDecision(
  replies: TranscriptBlock[],
  approvalId: string | null,
): string | null {
  for (const block of replies) {
    const meta = metaRecord(block);
    const decision = meta.decision;
    if (typeof decision !== "string" || !decision.trim()) continue;
    if (approvalId) {
      const id = typeof meta.approval_id === "string" ? meta.approval_id : null;
      if (id && id !== approvalId) continue;
    }
    if (
      meta.source === "approval_resolved" ||
      block.block_type === "system_notice" ||
      typeof meta.approval_id === "string"
    ) {
      return decision.trim();
    }
  }
  // Fallback: title/body language from log-parser style events.
  for (const block of replies) {
    if (block.block_type !== "system_notice") continue;
    const blob = `${block.title} ${block.body}`.toLowerCase();
    if (blob.includes("denied") || blob.includes("deny")) return "deny";
    if (blob.includes("approved") || blob.includes("allow")) return "allow_once";
  }
  return null;
}

function selectedFromApprovalDecision(decision: string | null): string[] {
  if (!decision) return [];
  if (/deny|denied|reject/i.test(decision)) return [APPROVAL_DENY_LABEL];
  if (/allow|approve/i.test(decision)) return [APPROVAL_ALLOW_LABEL];
  return [];
}

function pushNodeAt(
  nodes: Node<TraceGraphNodeData>[],
  id: string,
  x: number,
  y: number,
  data: TraceGraphNodeData,
): void {
  nodes.push({
    id,
    type: "trace",
    position: { x, y },
    data,
    draggable: false,
    connectable: false,
  });
}

function link(
  edges: Edge[],
  prevId: string | null,
  nextId: string,
  opts?: { dashed?: boolean; chosen?: boolean; muted?: boolean },
): string {
  if (prevId) {
    const dashed = Boolean(opts?.dashed || opts?.muted);
    const chosen = Boolean(opts?.chosen);
    edges.push({
      id: `e-${prevId}-${nextId}`,
      source: prevId,
      target: nextId,
      animated: chosen,
      style: dashed
        ? {
            strokeDasharray: "5 4",
            stroke: "var(--color-outline, #94a3b8)",
            opacity: 0.55,
          }
        : chosen
          ? { stroke: "#2563eb", strokeWidth: 2 }
          : { stroke: "var(--color-outline-variant, #cbd5e1)" },
    });
  }
  return nextId;
}

function toolStatus(step: ToolStep): TraceNodeStatus {
  if (toolStepRunning(step)) return "running";
  if (toolStepFailed(step)) return "failed";
  if (step.result) return "ok";
  return "neutral";
}

function shortId(id: string, max = 8): string {
  const t = id.trim();
  if (t.length <= max) return t;
  return `${t.slice(0, max)}…`;
}

/** Skill / Agent subtitle enrichment; falls back to command + duration. */
function enrichedToolSubtitle(step: ToolStep, toolName: string): string | undefined {
  const callBody = parseJsonBody(step.call?.body);
  const resultBody = parseJsonBody(step.result?.body);
  const duration = formatDurationMs(
    (step.result?.meta ?? step.call?.meta ?? {}) as Record<string, unknown>,
  );
  const parts: string[] = [];

  if (toolName === "Skill" || toolName === "SkillSearch") {
    const skillName =
      (typeof callBody?.name === "string" && callBody.name) ||
      (typeof resultBody?.skill === "string" && resultBody.skill) ||
      (typeof resultBody?.name === "string" && resultBody.name) ||
      "";
    if (skillName) parts.push(String(skillName));
    const mode = resultBody?.mode;
    if (typeof mode === "string" && mode.trim()) parts.push(mode.trim());
    const code = resultBody?.code;
    if (typeof code === "number") parts.push(`exit ${code}`);
  } else if (toolName === "Agent" || toolName === "Task") {
    const agentType =
      (typeof callBody?.agent_type === "string" && callBody.agent_type) ||
      (typeof callBody?.subagent_type === "string" && callBody.subagent_type) ||
      (typeof resultBody?.agent_type === "string" && resultBody.agent_type) ||
      (typeof resultBody?.subagent_type_resolved === "string" &&
        resultBody.subagent_type_resolved) ||
      "";
    if (agentType) parts.push(String(agentType));
    const status = resultBody?.status;
    if (typeof status === "string" && status.trim()) parts.push(status.trim());
    const nested =
      (typeof resultBody?.nested_task_id === "string" && resultBody.nested_task_id) ||
      (typeof resultBody?.agent_id === "string" && resultBody.agent_id) ||
      "";
    if (nested) parts.push(`id ${shortId(String(nested))}`);
  } else {
    const command = extractToolCommand(step.call) || extractToolCommand(step.result);
    if (command) parts.push(previewText(command, 48));
  }

  if (duration) parts.push(duration);
  return parts.length > 0 ? parts.join(" · ") : undefined;
}

/** Merge call+result into one tool node. */
function appendMergedToolStep(
  nodes: Node<TraceGraphNodeData>[],
  edges: Edge[],
  prevId: string | null,
  step: ToolStep,
  cursor: LayoutCursor,
): string | null {
  const primary = step.result ?? step.call;
  if (!primary) return prevId;

  const status = toolStatus(step);
  const id = step.call?.id ?? step.result?.id ?? `tool-${step.key}`;
  const label = toolDisplayName(primary);

  pushNodeAt(nodes, id, NODE_X, cursor.y, {
    kind: "tool",
    label,
    subtitle: enrichedToolSubtitle(step, label),
    block: primary,
    status,
    failed: status === "failed",
    running: status === "running",
  });
  cursor.y += NODE_GAP_Y;
  return link(edges, prevId, id);
}

function labelMatchesSelected(label: string, selectedSet: Set<string>): boolean {
  const lower = label.toLowerCase();
  if (selectedSet.has(lower)) return true;
  return [...selectedSet].some((s) => lower.includes(s) || s.includes(lower));
}

function appendLabeledFork(
  nodes: Node<TraceGraphNodeData>[],
  edges: Edge[],
  prevId: string | null,
  cursor: LayoutCursor,
  spec: ForkSpec,
): string | null {
  const selectedSet = new Set(spec.selected.map((s) => s.toLowerCase()));
  const settled = spec.selected.length > 0;

  pushNodeAt(nodes, spec.decisionId, NODE_X, cursor.y, {
    kind: "decision",
    label: previewText(spec.label, 40),
    subtitle: spec.subtitle,
    block: spec.block,
    status: settled ? "ok" : "running",
    running: Boolean(spec.running) || !settled,
  });
  link(edges, prevId, spec.decisionId);
  cursor.y += NODE_GAP_Y;

  if (spec.options.length === 0) {
    return spec.decisionId;
  }

  const branchY = cursor.y;
  const n = spec.options.length;
  const chosenIds: string[] = [];

  spec.options.forEach((label, i) => {
    const chosen = settled ? labelMatchesSelected(label, selectedSet) : false;
    const offset = n === 1 ? 0 : (i - (n - 1) / 2) * BRANCH_SPREAD_X;
    const branchId = `${spec.decisionId}:opt:${i}`;
    pushNodeAt(nodes, branchId, NODE_X + offset, branchY, {
      kind: "branch",
      label: previewText(label, 28),
      status: chosen ? "chosen" : settled ? "skipped" : "neutral",
      chosen,
      block: spec.block,
      branchKey: spec.branchKeys?.[i],
    });
    link(edges, spec.decisionId, branchId, {
      dashed: !chosen && settled,
      muted: !chosen && settled,
      chosen,
    });
    if (chosen) chosenIds.push(branchId);
  });

  cursor.y = branchY + NODE_GAP_Y;

  if (chosenIds.length === 1) {
    return chosenIds[0]!;
  }
  if (chosenIds.length > 1) {
    const mergeId = `${spec.decisionId}:merge`;
    pushNodeAt(nodes, mergeId, NODE_X, cursor.y, {
      kind: "branch",
      label: spec.selected.join(", "),
      status: "chosen",
      chosen: true,
      block: spec.block,
    });
    for (const cid of chosenIds) {
      link(edges, cid, mergeId, { chosen: true });
    }
    cursor.y += NODE_GAP_Y;
    return mergeId;
  }

  return null;
}

function appendDecisionFork(
  nodes: Node<TraceGraphNodeData>[],
  edges: Edge[],
  prevId: string | null,
  step: ToolStep,
  cursor: LayoutCursor,
  replies: TranscriptBlock[],
): string | null {
  const options = mergeOptionsFromQuestionRequests(step, replies);
  const selected = selectedLabelsFromStep(step);
  const decisionId = step.call?.id ?? `decision-${step.key}`;
  const primary = step.call ?? step.result;

  return appendLabeledFork(nodes, edges, prevId, cursor, {
    decisionId,
    label: decisionTitle(step),
    subtitle: options.length > 0 ? `${options.length} options` : undefined,
    block: primary,
    options,
    selected,
    running: selected.length === 0 && toolStepRunning(step),
  });
}

function appendQuestionRequestFork(
  nodes: Node<TraceGraphNodeData>[],
  edges: Edge[],
  prevId: string | null,
  block: TranscriptBlock,
  cursor: LayoutCursor,
): string | null {
  const meta = metaRecord(block);
  const options = optionLabelsFromValue(meta.options);
  const header =
    (typeof meta.header === "string" && meta.header.trim()) ||
    block.title.trim() ||
    "Question";
  const questionId =
    (typeof meta.question_id === "string" && meta.question_id.trim()) || block.id;

  return appendLabeledFork(nodes, edges, prevId, cursor, {
    decisionId: `question:${questionId}`,
    label: header,
    subtitle:
      options.length > 0
        ? `${options.length} options`
        : previewText(block.body || "", 40) || undefined,
    block,
    options,
    selected: [],
    running: true,
  });
}

function appendApprovalFork(
  nodes: Node<TraceGraphNodeData>[],
  edges: Edge[],
  prevId: string | null,
  block: TranscriptBlock,
  cursor: LayoutCursor,
  replies: TranscriptBlock[],
): string | null {
  const meta = metaRecord(block);
  const approvalId = approvalIdOf(block);
  const tool =
    (typeof meta.tool === "string" && meta.tool.trim()) ||
    (typeof meta.name === "string" && meta.name.trim()) ||
    block.title.replace(/^Approve\s+/i, "").trim() ||
    "Tool";
  const decision = findApprovalDecision(replies, approvalId);
  const selected = selectedFromApprovalDecision(decision);
  const preview =
    (typeof meta.input_preview === "string" && meta.input_preview.trim()) ||
    previewText(block.body || "", 40);

  return appendLabeledFork(nodes, edges, prevId, cursor, {
    decisionId: `approval:${approvalId ?? block.id}`,
    label: `Approve ${tool}`,
    subtitle: preview || undefined,
    block,
    options: [APPROVAL_ALLOW_LABEL, APPROVAL_DENY_LABEL],
    selected,
    running: selected.length === 0,
    branchKeys: ["allow", "deny"],
  });
}

function appendAssistantLike(
  nodes: Node<TraceGraphNodeData>[],
  edges: Edge[],
  prevId: string | null,
  block: TranscriptBlock,
  cursor: LayoutCursor,
): string | null {
  const body = block.body?.trim() ?? "";
  const live = Boolean(block.meta?.live);
  if (!body && !live && block.block_type !== "session_error") {
    return prevId;
  }
  const failed = block.block_type === "session_error";
  pushNodeAt(nodes, block.id, NODE_X, cursor.y, {
    kind: "assistant",
    label: failed ? "Error" : "Assistant",
    subtitle: body ? previewText(body) : "…",
    block,
    live,
    failed,
    status: failed ? "failed" : live ? "running" : "neutral",
  });
  cursor.y += NODE_GAP_Y;
  return link(edges, prevId, block.id);
}

function isApprovalResolvedNotice(block: TranscriptBlock): boolean {
  const meta = metaRecord(block);
  if (meta.source === "approval_resolved") return true;
  return (
    block.block_type === "system_notice" &&
    typeof meta.decision === "string" &&
    typeof meta.approval_id === "string"
  );
}

function isQuestionResolvedNotice(block: TranscriptBlock): boolean {
  return metaRecord(block).source === "question_resolved";
}

/**
 * Build a vertical turn DAG with merged tool nodes, AskUserQuestion /
 * question_request forks, and approval binary forks.
 */
export function buildExecutionTraceGraph(
  blocks: TranscriptBlock[],
): ExecutionTraceGraphModel {
  const nodes: Node<TraceGraphNodeData>[] = [];
  const edges: Edge[] = [];
  let prevId: string | null = null;
  const cursor: LayoutCursor = { y: 0 };

  const turns = blocksToTurns(blocks);
  for (let turnIndex = 0; turnIndex < turns.length; turnIndex += 1) {
    const turn = turns[turnIndex]!;
    const userId = turn.user.id;
    pushNodeAt(nodes, userId, NODE_X, cursor.y, {
      kind: "user",
      label: "User",
      subtitle: previewText(turn.user.body || turn.user.title || ""),
      block: turn.user,
      status: "neutral",
    });
    cursor.y += NODE_GAP_Y;
    prevId = link(edges, prevId, userId, { dashed: turnIndex > 0 });

    const hasAskTool = turnHasAskUserTool(turn.replies);
    let toolBuffer: TranscriptBlock[] = [];
    const flushTools = () => {
      if (toolBuffer.length === 0) return;
      for (const step of buildToolSteps(toolBuffer)) {
        if (isInteractiveToolName(toolNameFromStep(step))) {
          prevId = appendDecisionFork(nodes, edges, prevId, step, cursor, turn.replies);
        } else {
          prevId = appendMergedToolStep(nodes, edges, prevId, step, cursor);
        }
      }
      toolBuffer = [];
    };

    for (const block of turn.replies) {
      if (isToolBlock(block)) {
        toolBuffer.push(block);
        continue;
      }

      // question_request is covered by AskUserQuestion tool when present —
      // skip so we don't flush/double-draw mid-tool.
      if (block.block_type === "question_request") {
        if (hasAskTool) continue;
        flushTools();
        prevId = appendQuestionRequestFork(nodes, edges, prevId, block, cursor);
        continue;
      }

      if (block.block_type === "approval_request") {
        flushTools();
        prevId = appendApprovalFork(nodes, edges, prevId, block, cursor, turn.replies);
        continue;
      }

      // Resolved notices are consumed by the fork helpers; don't draw as assistant.
      if (isApprovalResolvedNotice(block) || isQuestionResolvedNotice(block)) {
        continue;
      }

      flushTools();
      if (
        block.block_type === "assistant_message" ||
        block.block_type === "progress_update" ||
        block.block_type === "session_error" ||
        (block.block_type === "system_notice" &&
          (block.meta?.source === "intermediate_assistant" ||
            block.meta?.source === "thinking_delta" ||
            block.meta?.source === "llm_start"))
      ) {
        prevId = appendAssistantLike(nodes, edges, prevId, block, cursor);
      }
    }
    flushTools();
  }

  return { nodes, edges };
}

/** Find the latest running tool/decision node id (for pulse styling). */
export function findRunningToolNodeId(
  nodes: Node<TraceGraphNodeData>[],
): string | null {
  for (let i = nodes.length - 1; i >= 0; i -= 1) {
    const n = nodes[i]!;
    if (
      (n.data.kind === "tool" || n.data.kind === "decision") &&
      (n.data.running || n.data.status === "running")
    ) {
      return n.id;
    }
  }
  return null;
}

export { toolStepKey };
