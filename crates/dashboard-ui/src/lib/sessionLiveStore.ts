import type { PendingApprovalsResponse, PendingQuestionsResponse, TranscriptBlock } from "@/api/types";
import type { ChatStreamEvent } from "@/lib/liveTranscript";

export type TurnPhase =
  | "waiting_first_token"
  | "streaming"
  | "running_tools"
  | null;

export type LivePendingQuestion = {
  question_id: string;
  session_id: string;
  question: string;
  header: string;
  options: Array<{ label: string; description?: string }>;
  multi_select: boolean;
};

export type LivePendingApproval = {
  approval_id: string;
  session_id: string;
  tool: string;
  input_preview: string;
};

export type SessionLiveState = {
  turnPhase: TurnPhase;
  turnPhaseStartedAt: string | null;
  pendingQuestions: LivePendingQuestion[];
  pendingApprovals: LivePendingApproval[];
};

const EMPTY: SessionLiveState = {
  turnPhase: null,
  turnPhaseStartedAt: null,
  pendingQuestions: [],
  pendingApprovals: [],
};

function metaString(meta: Record<string, unknown> | undefined, key: string): string {
  const v = meta?.[key];
  if (v === undefined || v === null) return "";
  return String(v);
}

function resolvedQuestionIds(blocks: TranscriptBlock[], events: ChatStreamEvent[]): Set<string> {
  const ids = new Set<string>();
  for (const block of blocks) {
    if (block.meta?.source === "question_resolved") {
      const id = metaString(block.meta, "question_id");
      if (id) ids.add(id);
    }
  }
  for (const evt of events) {
    if (evt.kind === "question_resolved") {
      const id = metaString(evt.payload, "question_id") || metaString(evt.block?.meta, "question_id");
      if (id) ids.add(id);
    }
  }
  return ids;
}

function resolvedApprovalIds(blocks: TranscriptBlock[], events: ChatStreamEvent[]): Set<string> {
  const ids = new Set<string>();
  for (const block of blocks) {
    if (block.meta?.source === "approval_resolved") {
      const id = metaString(block.meta, "approval_id");
      if (id) ids.add(id);
    }
  }
  for (const evt of events) {
    if (evt.kind === "approval_resolved") {
      const id = metaString(evt.payload, "approval_id") || metaString(evt.block?.meta, "approval_id");
      if (id) ids.add(id);
    }
  }
  return ids;
}

function questionFromBlock(block: TranscriptBlock): LivePendingQuestion | null {
  if (block.block_type !== "question_request") return null;
  const meta = block.meta ?? {};
  const question_id = metaString(meta, "question_id");
  if (!question_id) return null;
  const optionsRaw = meta.options;
  const options = Array.isArray(optionsRaw)
    ? optionsRaw
        .map((o) => {
          if (!o || typeof o !== "object") return null;
          const row = o as Record<string, unknown>;
          const label = metaString(row, "label");
          if (!label) return null;
          return {
            label,
            description: metaString(row, "description") || undefined,
          };
        })
        .filter(Boolean) as LivePendingQuestion["options"]
    : [];
  return {
    question_id,
    session_id: metaString(meta, "session_id"),
    question: block.body || metaString(meta, "question"),
    header: block.title || metaString(meta, "header"),
    options,
    multi_select: Boolean(meta.multi_select),
  };
}

function approvalFromBlock(block: TranscriptBlock): LivePendingApproval | null {
  if (block.block_type !== "approval_request") return null;
  const meta = block.meta ?? {};
  const approval_id = metaString(meta, "approval_id");
  if (!approval_id) return null;
  return {
    approval_id,
    session_id: metaString(meta, "session_id"),
    tool: metaString(meta, "tool") || block.title.replace(/^Approve\s*/i, ""),
    input_preview: block.body || metaString(meta, "input_preview"),
  };
}

function rehydratedQuestion(row: PendingQuestionsResponse["pending"][number]): LivePendingQuestion {
  return {
    question_id: row.question_id,
    session_id: row.session_id,
    question: row.question,
    header: row.header,
    options: row.options.map((o) => ({ label: o.label, description: o.description || undefined })),
    multi_select: row.multi_select,
  };
}

function rehydratedApproval(row: PendingApprovalsResponse["pending"][number]): LivePendingApproval {
  return {
    approval_id: row.approval_id,
    session_id: row.session_id,
    tool: row.tool,
    input_preview: row.input_preview,
  };
}

export function deriveTurnPhase(
  blocks: TranscriptBlock[],
  liveEvents: ChatStreamEvent[],
): { phase: TurnPhase; startedAt: string | null } {
  let phase: TurnPhase = null;
  let startedAt: string | null = null;

  const rank = (p: TurnPhase): number =>
    p === "running_tools" ? 3 : p === "streaming" ? 2 : p === "waiting_first_token" ? 1 : 0;

  const consider = (next: TurnPhase, at: string) => {
    if (!next) return;
    if (rank(next) >= rank(phase)) {
      phase = next;
      startedAt = at;
    }
  };

  for (const block of blocks) {
    if (block.meta?.source === "turn_phase") {
      consider(metaString(block.meta, "phase") as TurnPhase, block.at);
    } else if (block.meta?.source === "llm_start") {
      consider("waiting_first_token", block.at);
    } else if (block.block_type === "tool_call" && block.meta?.phase === "start") {
      consider("running_tools", block.at);
    } else if (block.block_type === "assistant_message" && block.meta?.live) {
      consider("streaming", block.at);
    }
  }

  for (const evt of liveEvents) {
    if (evt.kind === "turn_phase") {
      consider(metaString(evt.payload, "phase") as TurnPhase, evt.at);
    } else if (evt.kind === "llm_start") {
      consider("waiting_first_token", evt.at);
    } else if (evt.kind === "tool_start") {
      consider("running_tools", evt.at);
    } else if (evt.kind === "assistant_delta" || evt.kind === "assistant_done") {
      consider("streaming", evt.at);
    }
  }

  return { phase, startedAt };
}

export function deriveSessionLiveState(
  blocks: TranscriptBlock[],
  liveEvents: ChatStreamEvent[],
  rehydrateQuestions: PendingQuestionsResponse["pending"] = [],
  rehydrateApprovals: PendingApprovalsResponse["pending"] = [],
  sessionId?: string,
  sessionRunning = false,
  /** Optimistic resolutions (respond onMutate) not yet mirrored by SSE. */
  optimisticResolvedApprovalIds: Iterable<string> = [],
): SessionLiveState {
  const resolvedQ = resolvedQuestionIds(blocks, liveEvents);
  const resolvedA = resolvedApprovalIds(blocks, liveEvents);
  for (const id of optimisticResolvedApprovalIds) {
    const trimmed = id.trim();
    if (trimmed) resolvedA.add(trimmed);
  }

  const questionMap = new Map<string, LivePendingQuestion>();
  for (const row of rehydrateQuestions) {
    if (sessionId && row.session_id !== sessionId) continue;
    if (!resolvedQ.has(row.question_id)) {
      questionMap.set(row.question_id, rehydratedQuestion(row));
    }
  }
  for (const block of blocks) {
    const q = questionFromBlock(block);
    if (q && !resolvedQ.has(q.question_id)) {
      questionMap.set(q.question_id, q);
    }
  }
  for (const evt of liveEvents) {
    if (evt.kind === "question_request" && evt.block) {
      const q = questionFromBlock(evt.block);
      if (q && !resolvedQ.has(q.question_id)) {
        questionMap.set(q.question_id, q);
      }
    }
  }

  const approvalMap = new Map<string, LivePendingApproval>();
  for (const row of rehydrateApprovals) {
    if (sessionId && row.session_id !== sessionId) continue;
    if (!resolvedA.has(row.approval_id)) {
      approvalMap.set(row.approval_id, rehydratedApproval(row));
    }
  }
  for (const block of blocks) {
    const a = approvalFromBlock(block);
    if (a && !resolvedA.has(a.approval_id)) {
      approvalMap.set(a.approval_id, a);
    }
  }
  for (const evt of liveEvents) {
    if (evt.kind === "approval_request" && evt.block) {
      const a = approvalFromBlock(evt.block);
      if (a && !resolvedA.has(a.approval_id)) {
        approvalMap.set(a.approval_id, a);
      }
    }
  }

  const { phase, startedAt } = deriveTurnPhase(
    sessionRunning ? blocks : [],
    sessionRunning ? liveEvents : [],
  );

  return {
    turnPhase: phase,
    turnPhaseStartedAt: startedAt,
    pendingQuestions: [...questionMap.values()],
    pendingApprovals: [...approvalMap.values()],
  };
}

export function emptySessionLiveState(): SessionLiveState {
  return EMPTY;
}
