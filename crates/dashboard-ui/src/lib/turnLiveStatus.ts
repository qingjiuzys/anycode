import type { TurnPhase } from "@/lib/sessionLiveStore";
import { isProgressBlock } from "@/lib/progressMeta";
import { isFinalAssistantMessage } from "@/lib/agentActivitySummary";
import type { TurnReplyItem, ToolStep } from "@/lib/transcriptGrouping";
import { toolClusterSegmentActive, toolClusterSegmentSettled } from "@/lib/toolTraceState";

export const STALL_WARN_SECONDS = 60;
export const LONG_WAIT_SECONDS = 15;
export const VERY_LONG_WAIT_SECONDS = 30;

export type TurnLiveStatusInput = {
  isLast: boolean;
  isRunning: boolean;
  streamHasActivity?: boolean;
  turnStartedAt: string;
  turnEndedAt?: string | null;
  replyItems: TurnReplyItem[];
  turnPhase?: TurnPhase | null;
  pendingQuestionsCount?: number;
  pendingApprovalsCount?: number;
};

export type TurnLiveStatus = {
  turnStartedAt: string;
  turnEndedAt: string | null;
  phase: TurnPhase | null;
  hasAssistantText: boolean;
  hasProgressContent: boolean;
  hasFinalAssistantText: boolean;
  hasRunningTool: boolean;
  hasActiveToolCluster: boolean;
  hasThinkingCluster: boolean;
  allToolSteps: ToolStep[];
  waitingForUser: boolean;
  /** Primary live status row (replaces TurnPhaseBanner + TypingIndicator). */
  showLiveRecap: boolean;
  /** Duration-only recap when assistant text is already visible. */
  recapCompact: boolean;
  /** Hide per-cluster AgentActivityLine on last running turn. */
  showClusterActivityLine: boolean;
  /** Legacy typing dots — only when no phase and no tool activity. */
  showTypingIndicator: boolean;
  showThinkingLine: boolean;
  hideComposerWaiting: boolean;
  showStallActions: boolean;
};

function collectToolSteps(items: TurnReplyItem[]): ToolStep[] {
  const steps: ToolStep[] = [];
  for (const item of items) {
    if (item.kind === "tool_cluster") {
      steps.push(...item.steps);
    }
  }
  return steps;
}

function hasRunningToolInSteps(steps: ToolStep[]): boolean {
  return steps.some((step) => {
    const phase = step.call?.meta?.phase;
    return phase === "start" || (step.call && !step.result);
  });
}

function hasActiveCluster(
  items: TurnReplyItem[],
  isLast: boolean,
  isRunning: boolean,
): boolean {
  for (let i = 0; i < items.length; i += 1) {
    const item = items[i];
    if (item.kind !== "tool_cluster") continue;
    const lastClusterIndex = items.reduce(
      (acc, row, idx) => (row.kind === "tool_cluster" ? idx : acc),
      -1,
    );
    const segmentSettled = toolClusterSegmentSettled(items, i);
    const segmentActive = toolClusterSegmentActive(
      item.steps,
      isRunning,
      isLast && i === lastClusterIndex,
      segmentSettled,
    );
    if (segmentActive) return true;
  }
  return false;
}

export function deriveTurnLiveStatus(input: TurnLiveStatusInput): TurnLiveStatus {
  const {
    isLast,
    isRunning,
    streamHasActivity = false,
    turnStartedAt,
    turnEndedAt = null,
    replyItems,
    turnPhase = null,
    pendingQuestionsCount = 0,
    pendingApprovalsCount = 0,
  } = input;

  const allToolSteps = collectToolSteps(replyItems);
  const hasRunningTool = hasRunningToolInSteps(allToolSteps);
  const hasActiveToolCluster =
    isLast && isRunning && hasActiveCluster(replyItems, isLast, isRunning);
  const hasProgressContent = replyItems.some(
    (item) => item.kind === "block" && isProgressBlock(item.block),
  );
  const hasFinalAssistantText = replyItems.some(
    (item) =>
      item.kind === "block" &&
      item.block.block_type === "assistant_message" &&
      isFinalAssistantMessage(item.block) &&
      item.block.body.trim().length > 0,
  );
  const hasAssistantText = hasProgressContent || hasFinalAssistantText;
  const hasThinkingCluster = replyItems.some(
    (item) => item.kind === "tool_cluster" && item.processMessageCount > 0,
  );
  const hasToolCluster = replyItems.some((item) => item.kind === "tool_cluster");

  const live = isLast && isRunning;
  const waitingForUser =
    live && (pendingQuestionsCount > 0 || pendingApprovalsCount > 0);
  const recapCompact = live && hasProgressContent;

  const showLiveRecap =
    live &&
    !hasAssistantText &&
    (Boolean(turnPhase) ||
      hasActiveToolCluster ||
      hasRunningTool ||
      streamHasActivity ||
      hasThinkingCluster);

  const showTypingIndicator =
    live &&
    !hasAssistantText &&
    !showLiveRecap &&
    !streamHasActivity &&
    !hasToolCluster;

  const showThinkingLine =
    live &&
    streamHasActivity &&
    !hasAssistantText &&
    !turnPhase &&
    !hasRunningTool &&
    !hasThinkingCluster &&
    !hasToolCluster;

  const hideComposerWaiting =
    live &&
    (streamHasActivity ||
      turnPhase !== null ||
      hasActiveToolCluster ||
      hasRunningTool ||
      showLiveRecap);

  const showStallActions =
    live && !hasRunningTool && !hasActiveToolCluster && !waitingForUser;
  const showClusterActivityLine = !(live && isLast);

  return {
    turnStartedAt,
    turnEndedAt,
    phase: turnPhase,
    hasAssistantText,
    hasProgressContent,
    hasFinalAssistantText,
    hasRunningTool,
    hasActiveToolCluster,
    hasThinkingCluster,
    allToolSteps,
    waitingForUser,
    showLiveRecap,
    recapCompact,
    showClusterActivityLine,
    showTypingIndicator,
    showThinkingLine,
    hideComposerWaiting,
    showStallActions,
  };
}

export function hideComposerWaitingFromSession(opts: {
  running: boolean;
  streamHasActivity: boolean;
  turnPhase: TurnPhase | null;
  liveBlocksActive: boolean;
}): boolean {
  if (!opts.running) return false;
  return (
    opts.streamHasActivity ||
    opts.turnPhase !== null ||
    opts.liveBlocksActive
  );
}

export function turnEndedAtFromReplies(
  replies: { at: string }[],
  fallback: string,
): string | null {
  if (replies.length === 0) return null;
  let latest = fallback;
  let latestTs = Number.NaN;
  for (const block of replies) {
    const ts = Date.parse(
      block.at.includes("T") ? block.at : block.at.replace(" ", "T") + "Z",
    );
    if (!Number.isNaN(ts) && (Number.isNaN(latestTs) || ts > latestTs)) {
      latestTs = ts;
      latest = block.at;
    }
  }
  return latest;
}
