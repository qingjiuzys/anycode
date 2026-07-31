import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api } from "@/api/client";
import type { TranscriptBlock } from "@/api/types";
import { CopyButton } from "@/components/ui/CopyButton";
import { SpeakButton } from "@/components/SpeakButton";
import { Icon } from "@/components/Icon";
import {
  isCommandBlock,
  TranscriptCommandBlock,
} from "@/components/TranscriptCommandBlock";
import { TranscriptMarkdown } from "@/components/TranscriptMarkdown";
import { ToolTraceCluster } from "@/components/chat/ToolTraceCluster";
import { TaskReceiptCard } from "@/components/chat/TaskReceiptCard";
import { isStatusMessage } from "@/lib/agentActivitySummary";
import { isTaskSummaryReceipt } from "@/lib/taskSummaryReceipt";
import {
  CollapsiblePanel,
  previewLines,
  useContentCollapse,
} from "@/components/ui/CollapsiblePanel";
import { formatRelativeTime } from "@/utils/formatTime";
import { formatTranscriptBlockTitle } from "@/lib/eventFormat";
import { groupTurnReplies, mergeFinalAssistantBlocks } from "@/lib/transcriptGrouping";
import { dedupeNarrationWithProgress } from "@/lib/phaseGrouping";
import {
  groupTurnForWorkLog,
  latestWorkSummary,
} from "@/lib/workLogGrouping";
import {
  isProgressBlock,
  progressDiscovery,
  progressNext,
  progressSummary,
} from "@/lib/progressMeta";
import {
  resolveActiveReplySegment,
  resolveFinalAssistantIndex,
  toolClusterSegmentActive,
  toolClusterSegmentSettled,
} from "@/lib/toolTraceState";
import { sessionDetailSearch } from "@/lib/sessionLinks";
import {
  SESSION_QUERY_GC_MS,
  transcriptQueryOptions,
} from "@/lib/sessionQuery";
import { collectInlineDeliverables } from "@/lib/deliverableInference";
import { isArtifactScaffoldOnly } from "@/lib/artifactMarker";
import { sanitizeAssistantDisplay } from "@/lib/assistantText";
import {
  isScrollNearBottom,
  SCROLL_RESIZE_THROTTLE_MS,
  shouldSkipScrollToBottom,
  streamFollowSignature as buildStreamFollowSignature,
} from "@/lib/transcriptScroll";
import { humanizeTranscriptError } from "@/lib/transcriptError";
import { useSmoothText } from "@/hooks/useSmoothText";
import { resolveCanonicalTranscriptBlocks, hasTurnStreamActivity } from "@/lib/liveTranscript";
import { findActiveToolInExecutionLog, findActiveToolInReplies } from "@/lib/transcriptGrouping";
import { AskUserQuestionInbox } from "@/components/AskUserQuestionInbox";
import { TurnRecapHeader } from "@/components/TurnRecapHeader";
import {
  deriveTurnLiveStatus,
  turnEndedAtFromReplies,
} from "@/lib/turnLiveStatus";
import {
  interactiveStepHistoryLabel,
  isInteractiveToolCluster,
  shouldHideInteractiveCluster,
} from "@/lib/interactiveTools";
import {
  DeliverableCard,
  type DeliverableCardProps,
} from "@/components/deliverables/DeliverableCard";
import { useLocale, useT } from "@/i18n/context";
import type { SseStatus } from "@/hooks/useEventSource";
import type { SessionLiveState } from "@/lib/sessionLiveStore";

interface Props {
  sessionId: string | null;
  projectId?: string | null;
  modelName?: string | null;
  isRunning?: boolean;
  /** Canonical merge / polling stream mode (prefer over raw sseLive + chatStreamLive). */
  streamLive?: boolean;
  sseLive?: boolean;
  liveBlocks?: TranscriptBlock[];
  liveEvents?: import("@/lib/liveTranscript").ChatStreamEvent[];
  chatStreamLive?: boolean;
  sseStatus?: SseStatus;
  sessionLive?: SessionLiveState;
  questionsRespondAllowed?: boolean;
  approvalsRespondAllowed?: boolean;
  scrollContainerRef?: React.RefObject<HTMLElement | null>;
  /** Shown while transcript loads (from session list). */
  promptPreview?: string | null;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
}

type ConversationTurn = {
  id: string;
  user: TranscriptBlock;
  replies: TranscriptBlock[];
};

const VIRTUAL_TURN_THRESHOLD = 30;
const COMPACT_TURN_ESTIMATE_PX = 220;

function getScrollContainer(
  scrollContainerRef?: React.RefObject<HTMLElement | null>,
  localScrollRef?: React.RefObject<HTMLElement | null>,
): HTMLElement | null {
  return scrollContainerRef?.current ?? localScrollRef?.current ?? null;
}

export function ConversationTranscript({
  sessionId,
  projectId,
  modelName,
  isRunning,
  streamLive: streamLiveOverride,
  sseLive = false,
  liveBlocks = [],
  liveEvents = [],
  chatStreamLive = false,
  sseStatus = "offline",
  sessionLive,
  questionsRespondAllowed = true,
  approvalsRespondAllowed = true,
  scrollContainerRef,
  promptPreview,
  selectedToolId,
  onSelectTool,
}: Props) {
  const t = useT();
  const bottomRef = useRef<HTMLDivElement>(null);
  const localScrollRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const prevTurnCountRef = useRef(0);
  const userNearBottomRef = useRef(true);
  const scrollRafRef = useRef<number | null>(null);
  const resizeScrollAtRef = useRef(0);

  const running = Boolean(isRunning);
  const streamLive =
    streamLiveOverride ?? (chatStreamLive || (sseLive && running));
  const pollFallback = running && !streamLive && sseStatus === "offline";

  const transcript = useQuery({
    ...transcriptQueryOptions(sessionId!, running, chatStreamLive, streamLive),
    enabled: Boolean(sessionId),
    refetchInterval: pollFallback ? 30_000 : false,
    refetchIntervalInBackground: false,
    placeholderData: (prev) => prev,
  });

  const liveLog = useQuery({
    queryKey: ["session-execution-log-live", sessionId],
    queryFn: () => api.sessionExecutionLog(sessionId!, { offset: 0, limit: 120 }),
    enabled: Boolean(sessionId) && pollFallback,
    staleTime: 30_000,
    gcTime: SESSION_QUERY_GC_MS,
    placeholderData: (prev) => prev,
    refetchInterval: pollFallback ? 30_000 : false,
    refetchIntervalInBackground: false,
  });

  const blocks = useMemo(() => {
    const snapshot = transcript.data?.transcript.blocks ?? [];
    const snapshotMaxSeq = transcript.data?.transcript.max_seq ?? 0;
    return resolveCanonicalTranscriptBlocks(
      snapshot,
      liveEvents,
      snapshotMaxSeq,
      streamLive,
    );
  }, [
    liveEvents,
    streamLive,
    transcript.data?.transcript.blocks,
    transcript.data?.transcript.max_seq,
  ]);
  const lifecycleCount = transcript.data?.transcript.lifecycle?.length ?? 0;
  const turns = useMemo(() => blocksToTurns(blocks), [blocks]);
  const lastTurn = turns.length > 0 ? turns[turns.length - 1] : null;
  const activeToolFromReplies = useMemo(
    () => (lastTurn ? findActiveToolInReplies(lastTurn.replies) : null),
    [lastTurn],
  );
  const activeToolFromLog = useMemo(
    () => findActiveToolInExecutionLog(liveLog.data?.execution_log.lines ?? []),
    [liveLog.data?.execution_log.lines],
  );
  const activeTool = activeToolFromReplies ?? activeToolFromLog;
  const turnHasActivity = hasTurnStreamActivity(
    liveBlocks,
    activeTool,
    lastTurn?.replies ?? [],
  );

  const stalledSeconds = useStalledSeconds(
    Boolean(isRunning),
    `${blocks.length}:${liveLog.data?.execution_log.lines.length ?? 0}:${activeTool ?? ""}:${turnHasActivity}:${sessionLive?.pendingQuestions.length ?? 0}:${sessionLive?.pendingApprovals.length ?? 0}`,
  );
  const lastUserPrompt =
    turns.length > 0 ? turns[turns.length - 1].user.body : null;

  const useVirtual = turns.length >= VIRTUAL_TURN_THRESHOLD;
  const virtualParentRef = scrollContainerRef ?? localScrollRef;

  const virtualizer = useVirtualizer({
    count: turns.length,
    getScrollElement: () => virtualParentRef.current,
    estimateSize: () => COMPACT_TURN_ESTIMATE_PX,
    overscan: 4,
  });

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = "auto") => {
      if (useVirtual) {
        if (turns.length > 0) {
          virtualizer.scrollToIndex(turns.length - 1, { align: "end", behavior: "auto" });
        }
        return;
      }
      const container = getScrollContainer(scrollContainerRef, localScrollRef);
      if (container) {
        if (shouldSkipScrollToBottom(container)) {
          return;
        }
        container.scrollTo({ top: container.scrollHeight, behavior });
        return;
      }
      bottomRef.current?.scrollIntoView({ behavior, block: "end" });
    },
    [scrollContainerRef, turns.length, useVirtual, virtualizer],
  );

  const scheduleScrollToBottom = useCallback(() => {
    if (!userNearBottomRef.current) return;
    if (scrollRafRef.current !== null) return;
    scrollRafRef.current = requestAnimationFrame(() => {
      scrollRafRef.current = null;
      scrollToBottom("auto");
    });
  }, [scrollToBottom]);

  const streamFollowSignature = useMemo(
    () =>
      buildStreamFollowSignature({
        running,
        streamLive,
        blocksLength: blocks.length,
        liveEventsLength: liveEvents.length,
        turnHasActivity,
        turnPhase: sessionLive?.turnPhase ?? null,
        liveBlocksLength: liveBlocks.length,
      }),
    [
      blocks,
      liveBlocks.length,
      liveEvents.length,
      running,
      sessionLive?.turnPhase,
      streamLive,
      turnHasActivity,
    ],
  );

  useEffect(() => {
    const container = getScrollContainer(scrollContainerRef, localScrollRef);
    if (!container) return;
    const onScroll = () => {
      userNearBottomRef.current = isScrollNearBottom(container);
    };
    container.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
    return () => container.removeEventListener("scroll", onScroll);
  }, [scrollContainerRef, sessionId]);

  useEffect(() => {
    const grew = turns.length > prevTurnCountRef.current;
    prevTurnCountRef.current = turns.length;
    if (!grew && !isRunning) return;
    if (isRunning && !userNearBottomRef.current && !grew) return;

    scrollToBottom(isRunning ? "auto" : "smooth");
  }, [turns.length, isRunning, scrollToBottom]);

  useEffect(() => {
    if (!streamFollowSignature) return;
    scheduleScrollToBottom();
  }, [streamFollowSignature, scheduleScrollToBottom]);

  useEffect(() => {
    const content = contentRef.current;
    if (!content) return;
    if (!running && !streamLive) return;

    const ro = new ResizeObserver(() => {
      const now = Date.now();
      if (now - resizeScrollAtRef.current < SCROLL_RESIZE_THROTTLE_MS) {
        return;
      }
      resizeScrollAtRef.current = now;
      scheduleScrollToBottom();
    });
    ro.observe(content);
    return () => ro.disconnect();
  }, [running, streamLive, sessionId, scheduleScrollToBottom]);

  useEffect(
    () => () => {
      if (scrollRafRef.current !== null) {
        cancelAnimationFrame(scrollRafRef.current);
      }
    },
    [],
  );

  if (!sessionId) return null;

  const showColdLoading = transcript.isPending && !transcript.data;
  if (showColdLoading) {
    const preview = promptPreview?.trim();
    if (preview) {
      return (
        <div className="space-y-4 opacity-80">
          <div className="flex w-full justify-end">
            <div className="max-w-[min(100%,42rem)] rounded-2xl bg-surface-container-high px-4 py-3 text-sm">
              {preview}
            </div>
          </div>
          <p className="text-xs text-secondary m-0">{t("common.loading")}</p>
        </div>
      );
    }
    return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  }
  if (transcript.isError) {
    return (
      <p className="text-sm text-error">
        {(transcript.error as Error).message}
      </p>
    );
  }
  if (turns.length === 0 && !isRunning) {
    return (
      <div className="text-sm text-secondary space-y-2">
        <p className="m-0">{t("conversations.noMessages")}</p>
        {lifecycleCount > 0 && <ExecutionLogLink sessionId={sessionId} />}
      </div>
    );
  }

  const tail = (
    <>
      {lifecycleCount > 0 && <ExecutionLogLink sessionId={sessionId} />}
      <div ref={bottomRef} aria-hidden className="h-px shrink-0 scroll-pad-bottom" />
    </>
  );

  const fetchingBar =
    transcript.isFetching && transcript.data ? (
      <div
        className="h-0.5 w-full bg-primary/15 overflow-hidden mb-3 rounded-full shrink-0"
        aria-hidden
      >
        <div className="h-full w-1/3 bg-primary/60 animate-pulse rounded-full" />
      </div>
    ) : null;

  if (useVirtual) {
    return (
      <div ref={contentRef} className="conversation-transcript-content">
        {fetchingBar}
        <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((item) => {
          const turn = turns[item.index];
          return (
            <div
              key={turn.id}
              data-index={item.index}
              ref={virtualizer.measureElement}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${item.start}px)`,
              }}
            >
              <ConversationTurnView
                turn={turn}
                isLast={item.index === turns.length - 1}
                isRunning={Boolean(isRunning)}
                streamHasActivity={turnHasActivity}
                sessionId={sessionId}
                projectId={projectId ?? undefined}
                modelName={modelName}
                selectedToolId={selectedToolId}
                onSelectTool={onSelectTool}
                sessionLive={sessionLive}
                questionsRespondAllowed={questionsRespondAllowed}
                approvalsRespondAllowed={approvalsRespondAllowed}
                stallSeconds={stalledSeconds}
                lastUserPrompt={lastUserPrompt}
              />
            </div>
          );
        })}
        <div className="flex flex-col gap-8 pt-4">{tail}</div>
      </div>
      </div>
    );
  }

  return (
    <div ref={contentRef} className="conversation-transcript-content">
      {fetchingBar}
      <div className={`flex flex-col ${isRunning ? "gap-4" : "gap-5"}`}>
      {turns.map((turn, index) => (
        <ConversationTurnView
          key={turn.id}
          turn={turn}
          isLast={index === turns.length - 1}
          isRunning={Boolean(isRunning)}
          streamHasActivity={turnHasActivity}
          sessionId={sessionId}
          projectId={projectId ?? undefined}
          modelName={modelName}
          selectedToolId={selectedToolId}
          onSelectTool={onSelectTool}
          sessionLive={sessionLive}
          questionsRespondAllowed={questionsRespondAllowed}
          approvalsRespondAllowed={approvalsRespondAllowed}
          stallSeconds={stalledSeconds}
          lastUserPrompt={lastUserPrompt}
        />
      ))}
      {tail}
    </div>
    </div>
  );
}

function ConversationTurnView({
  turn,
  isLast,
  isRunning,
  streamHasActivity,
  sessionId,
  projectId,
  modelName,
  selectedToolId,
  onSelectTool,
  sessionLive,
  questionsRespondAllowed = true,
  approvalsRespondAllowed: _approvalsRespondAllowed = true,
  stallSeconds = 0,
  lastUserPrompt = null,
}: {
  turn: ConversationTurn;
  isLast: boolean;
  isRunning: boolean;
  streamHasActivity?: boolean;
  sessionId: string;
  projectId?: string;
  modelName?: string | null;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
  sessionLive?: SessionLiveState;
  questionsRespondAllowed?: boolean;
  approvalsRespondAllowed?: boolean;
  stallSeconds?: number;
  lastUserPrompt?: string | null;
}) {
  const t = useT();
  const locale = useLocale();
  const replyItems = useMemo(() => {
    const grouped = groupTurnReplies(mergeFinalAssistantBlocks(turn.replies));
    return dedupeNarrationWithProgress(grouped);
  }, [turn.replies]);
  const activeSegmentIndex = useMemo(
    () => resolveActiveReplySegment(replyItems, { isLast, isRunning }),
    [replyItems, isLast, isRunning],
  );
  const finalAssistantIndex = useMemo(
    () => resolveFinalAssistantIndex(replyItems),
    [replyItems],
  );
  // Recap header only — main transcript renders replyItems in timeline order.
  const workBundle = useMemo(() => groupTurnForWorkLog(replyItems), [replyItems]);
  const latestProgress = useMemo(
    () =>
      latestWorkSummary(workBundle.work, (block) =>
        progressSummary(block, sanitizeAssistantDisplay(block.body, locale)),
      ),
    [workBundle.work, locale],
  );
  const turnEndedAt = useMemo(
    () => turnEndedAtFromReplies(turn.replies, turn.user.at),
    [turn.replies, turn.user.at],
  );
  const pendingQuestionsCount = sessionLive?.pendingQuestions.length ?? 0;
  const pendingApprovalsCount = sessionLive?.pendingApprovals.length ?? 0;
  const liveStatus = useMemo(
    () =>
      deriveTurnLiveStatus({
        isLast,
        isRunning,
        streamHasActivity,
        turnStartedAt: turn.user.at,
        turnEndedAt,
        replyItems,
        turnPhase: isLast ? (sessionLive?.turnPhase ?? null) : null,
        pendingQuestionsCount,
        pendingApprovalsCount,
      }),
    [
      isLast,
      isRunning,
      streamHasActivity,
      turn.user.at,
      turnEndedAt,
      replyItems,
      sessionLive?.turnPhase,
      pendingQuestionsCount,
      pendingApprovalsCount,
    ],
  );

  const lastLiveAssistantBlockId = useMemo(() => {
    if (!isLast || !isRunning) {
      return null;
    }
    for (let i = replyItems.length - 1; i >= 0; i -= 1) {
      const item = replyItems[i]!;
      if (item.kind !== "block") continue;
      if (item.block.block_type === "assistant_message") {
        return item.block.id;
      }
    }
    return null;
  }, [replyItems, isLast, isRunning]);

  const markerDeliverablesByBlockId = useMemo(
    () => collectInlineDeliverables(replyItems, projectId),
    [replyItems, projectId],
  );
  const turnHasDeliverableBlocks = useMemo(
    () =>
      replyItems.some(
        (item) => item.kind === "block" && item.block.block_type === "deliverable",
      ),
    [replyItems],
  );

  const showRecapHeader = isLast && isRunning;
  const hideBubbleTimestamps = isLast && isRunning;
  const showInlineQuestionInbox =
    isLast && isRunning && pendingQuestionsCount > 0;

  return (
    <article className={`flex flex-col gap-2.5 ${isRunning && isLast ? "pb-4" : "pb-5"}`}>
      <MessageRow align="right">
        <UserBubble block={turn.user} hideTimestamp={hideBubbleTimestamps} />
      </MessageRow>

      {showRecapHeader && (
        <MessageRow align="left">
          <TurnRecapHeader
            turnStartedAt={turn.user.at}
            turnEndedAt={turnEndedAt}
            isRunning={isLast && isRunning}
            phase={isLast ? (sessionLive?.turnPhase ?? null) : null}
            toolSteps={liveStatus.allToolSteps}
            sessionId={isLast && isRunning ? sessionId : undefined}
            lastUserPrompt={isLast && isRunning ? lastUserPrompt : null}
            stallSeconds={isLast && isRunning ? stallSeconds : 0}
            showStallActions={liveStatus.showStallActions}
            compact={liveStatus.recapCompact}
            waitingForUser={liveStatus.waitingForUser}
            latestProgressSummary={latestProgress}
          />
        </MessageRow>
      )}

      {showInlineQuestionInbox && (
        <MessageRow align="left">
          <AskUserQuestionInbox
            sessionId={sessionId}
            hideWhenEmpty
            inline
            questions={sessionLive?.pendingQuestions ?? []}
            respondAllowed={questionsRespondAllowed}
          />
        </MessageRow>
      )}

      {replyItems.map((item, itemIndex) => {
        const segmentExpanded =
          itemIndex === activeSegmentIndex ||
          (!isRunning &&
            item.kind === "block" &&
            itemIndex === finalAssistantIndex);

        if (item.kind === "tool_cluster") {
          const hideInteractiveCluster = shouldHideInteractiveCluster({
            isLast,
            isRunning,
            steps: item.steps,
            pendingQuestionsCount,
            pendingApprovalsCount,
          });
          const showInteractiveHistory =
            item.steps.length > 0 &&
            isInteractiveToolCluster(item.steps) &&
            (!isLast || !isRunning);
          if (hideInteractiveCluster) {
            return null;
          }
          if (showInteractiveHistory) {
            return (
              <MessageRow key={item.id} align="left">
                <InteractiveToolHistoryLine steps={item.steps} />
              </MessageRow>
            );
          }
          if (item.steps.length === 0 && item.processSnippets.length === 0) {
            return null;
          }
          let lastClusterIndex = -1;
          for (let i = 0; i < replyItems.length; i++) {
            if (replyItems[i]?.kind === "tool_cluster") lastClusterIndex = i;
          }
          const settled = toolClusterSegmentSettled(replyItems, itemIndex);
          const clusterLive = toolClusterSegmentActive(
            item.steps,
            isLast && isRunning,
            itemIndex === lastClusterIndex,
            settled,
          );
          return (
            <MessageRow key={item.id} align="left">
              <ToolTraceCluster
                steps={item.steps}
                processMessageCount={item.processMessageCount}
                processSnippets={item.processSnippets}
                isRunning={clusterLive}
                selectedToolId={selectedToolId}
                onSelectTool={onSelectTool}
                suppressActivityLine
                defaultCollapsed={!clusterLive}
                forceExpanded={segmentExpanded && clusterLive}
              />
            </MessageRow>
          );
        }

        const block = item.block;
        if (shouldSkipApprovalBlock(block)) return null;
        if (isCommandBlock(block)) {
          return (
            <MessageRow key={block.id} align="left">
              <TranscriptCommandBlock block={block} />
            </MessageRow>
          );
        }
        if (
          isProgressBlock(block) ||
          block.block_type === "progress_update" ||
          (block.block_type === "system_notice" &&
            (block.meta?.source === "intermediate_assistant" ||
              block.meta?.source === "thinking_delta" ||
              block.meta?.source === "llm_start"))
        ) {
          const lineLive = Boolean(isLast && isRunning && block.meta?.live);
          return (
            <MessageRow key={block.id} align="left">
              <TimelineProgressLine
                block={block}
                live={lineLive}
                expanded={segmentExpanded}
              />
            </MessageRow>
          );
        }
        if (block.block_type === "deliverable") {
          const deliverable = deliverablePropsFromBlock(block);
          if (!deliverable) return null;
          return (
            <MessageRow key={block.id} align="left">
              <DeliverableCard {...deliverable} projectId={deliverable.projectId ?? projectId ?? undefined} />
            </MessageRow>
          );
        }
        if (block.block_type === "assistant_message") {
          const markerDeliverables = markerDeliverablesByBlockId.get(block.id) ?? [];
          const displayBody = sanitizeAssistantDisplay(block.body, locale);
          const scaffoldEcho =
            isArtifactScaffoldOnly(block.body) || isArtifactScaffoldOnly(displayBody);
          const hasDeliverables =
            markerDeliverables.length > 0 || turnHasDeliverableBlocks;
          if (!displayBody.trim() && !block.meta?.live && markerDeliverables.length === 0) {
            return null;
          }
          const isFinal =
            itemIndex === finalAssistantIndex ||
            (isLast && isRunning && block.meta?.live === true && itemIndex === activeSegmentIndex);
          const showReplyBubble =
            (displayBody.trim().length > 0 || Boolean(block.meta?.live)) &&
            !(scaffoldEcho && hasDeliverables && !block.meta?.live);
          if (!showReplyBubble && markerDeliverables.length === 0) {
            return null;
          }
          return (
            <MessageRow key={block.id} align="left">
              <div className="flex flex-col gap-3 w-full max-w-[min(100%,42rem)]">
                {showReplyBubble ? (
                  <ReplyBubble
                    block={block}
                    modelName={modelName}
                    showStreamCursor={block.id === lastLiveAssistantBlockId}
                    forceStreamSmooth={isLast && isRunning}
                    hideTimestamp={hideBubbleTimestamps}
                    collapsed={!segmentExpanded && !isFinal}
                  />
                ) : null}
                {markerDeliverables.map((deliverable) => (
                  <DeliverableCard
                    key={deliverable.path}
                    {...deliverable}
                    projectId={deliverable.projectId ?? projectId ?? undefined}
                  />
                ))}
              </div>
            </MessageRow>
          );
        }
        return null;
      })}

      {liveStatus.showThinkingLine && (
        <MessageRow align="left">
          <div className="chat-trace-line chat-trace-line-thinking">
            <div className="chat-trace-line-toggle">
              <Icon name="progress_activity" size={14} />
              <span>{t("conversations.thinkingWaiting")}</span>
            </div>
          </div>
        </MessageRow>
      )}

      {liveStatus.showTypingIndicator && (
        <MessageRow align="left">
          <TypingIndicator compact />
        </MessageRow>
      )}
    </article>
  );
}

function shouldSkipApprovalBlock(block: TranscriptBlock): boolean {
  if (block.block_type === "approval_request") {
    return true;
  }
  return (
    block.block_type === "system_notice" && block.meta?.source === "approval_resolved"
  );
}

function MessageRow({
  align,
  children,
}: {
  align: "left" | "right";
  children: React.ReactNode;
}) {
  return (
    <div
      className={`flex w-full ${align === "right" ? "justify-end" : "justify-start"}`}
    >
      <div className="max-w-[min(100%,48rem)] w-fit min-w-0">{children}</div>
    </div>
  );
}

function InteractiveToolHistoryLine({ steps }: { steps: import("@/lib/transcriptGrouping").ToolStep[] }) {
  const t = useT();
  const label =
    steps.map(interactiveStepHistoryLabel).find((value) => value && value.length > 0) ??
    "AskUserQuestion";
  return (
    <p className="interactive-tool-history m-0 text-xs text-secondary leading-snug">
      {t("conversations.interactiveToolAsked").replace("{label}", label)}
    </p>
  );
}

function TimelineProgressLine({
  block,
  live,
  expanded: expandedProp,
}: {
  block: TranscriptBlock;
  live: boolean;
  /** Accordion: parent controls whether this segment is open. */
  expanded: boolean;
}) {
  const t = useT();
  const locale = useLocale();
  const [userPinned, setUserPinned] = useState<boolean | null>(null);
  const expanded = userPinned ?? expandedProp;
  const summary = progressSummary(block, sanitizeAssistantDisplay(block.body, locale));
  const next = progressNext(block);
  const finding = progressDiscovery(block);
  const body =
    !summary && !next && !finding
      ? sanitizeAssistantDisplay(block.body, locale).trim()
      : "";
  if (!summary && !next && !finding && !body) return null;

  const preview = summary || body || finding || next || "";
  const mdBody = summary || body;

  useEffect(() => {
    // New accordion target clears manual pin.
    setUserPinned(null);
  }, [expandedProp]);

  if (!expanded) {
    return (
      <button
        type="button"
        className="agent-narration-fold"
        onClick={() => setUserPinned(true)}
        aria-expanded={false}
        aria-label={t("common.expand")}
      >
        <Icon name="chevron_right" size={18} className="transcript-expand-icon" />
        <span className="agent-narration-fold__text">{preview.replace(/\s+/g, " ")}</span>
      </button>
    );
  }

  return (
    <div className={`agent-work-line ${live ? "agent-work-line--live" : ""}`}>
      {!expandedProp || userPinned ? (
        <button
          type="button"
          className="agent-narration-fold agent-narration-fold--open"
          onClick={() => setUserPinned(false)}
          aria-expanded
          aria-label={t("common.collapse")}
        >
          <Icon name="expand_more" size={18} className="transcript-expand-icon" />
        </button>
      ) : null}
      {mdBody ? (
        <TranscriptMarkdown
          text={mdBody}
          live={live}
          className="agent-work-line__text"
        />
      ) : null}
      {finding ? (
        <p className="m-0 mt-1 agent-work-line__meta">
          <span className="font-medium">{t("conversations.progressDiscoveryPrefix")}</span>
          {finding}
        </p>
      ) : null}
      {next ? (
        <p className="m-0 mt-1 agent-work-line__meta">
          <span className="font-medium">{t("conversations.progressNextPrefix")}</span>
          {next}
        </p>
      ) : null}
    </div>
  );
}

type VisionImageMeta = { mime_type?: string; data_base64?: string };

/** Visible user text — strip legacy OCR dumps that used to be inlined into the body. */
function userBubbleDisplayText(body: string): string {
  const marker = "--- OCR from attached images";
  const idx = body.indexOf(marker);
  if (idx < 0) return body;
  return body.slice(0, idx).trimEnd();
}

function visionImagesFromMeta(meta: TranscriptBlock["meta"]): VisionImageMeta[] {
  const raw = meta?.vision_images;
  if (!Array.isArray(raw)) return [];
  return raw.filter(
    (img): img is VisionImageMeta =>
      !!img &&
      typeof img === "object" &&
      typeof (img as VisionImageMeta).mime_type === "string" &&
      typeof (img as VisionImageMeta).data_base64 === "string",
  );
}

function UserBubble({
  block,
  hideTimestamp = false,
}: {
  block: TranscriptBlock;
  hideTimestamp?: boolean;
}) {
  const t = useT();
  const isQueued =
    block.meta?.source === "message_queue" && block.meta?.status === "pending";
  const displayBody = userBubbleDisplayText(block.body);
  const visionImages = visionImagesFromMeta(block.meta);
  const copyText = displayBody || t("conversations.attachImage");
  return (
    <div
      className={`bubble-user rounded-2xl rounded-br-md px-4 py-3 text-sm shadow-sm group relative ${
        isQueued ? "opacity-80 border border-dashed border-outline-variant" : ""
      }`}
    >
      {isQueued && (
        <span className="inline-flex items-center rounded-full bg-surface-container-high px-2 py-0.5 text-[10px] text-secondary mb-2">
          {t("conversations.messageQueueLabel")}
        </span>
      )}
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
        <CopyButton text={copyText} label={t("conversations.copyMessage")} />
      </div>
      <div className="leading-relaxed">
        {visionImages.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-2">
            {visionImages.map((img, i) => (
              <img
                key={`${img.mime_type}-${i}`}
                src={`data:${img.mime_type};base64,${img.data_base64}`}
                alt=""
                className="max-h-40 max-w-[min(100%,16rem)] rounded-lg object-contain border border-white/20 bg-black/5"
              />
            ))}
          </div>
        )}
        {displayBody ? (
          <span className="whitespace-pre-wrap break-words">{displayBody}</span>
        ) : null}
        {!hideTimestamp && (
          <time className="ml-1.5 text-[11px] opacity-70 whitespace-nowrap">
            {formatRelativeTime(block.at)}
          </time>
        )}
      </div>
    </div>
  );
}

const ReplyBubble = memo(function ReplyBubble({
  block,
  modelName: _modelName,
  showStreamCursor = false,
  forceStreamSmooth = false,
  hideTimestamp = false,
  collapsed = false,
}: {
  block: TranscriptBlock;
  modelName?: string | null;
  showStreamCursor?: boolean;
  forceStreamSmooth?: boolean;
  hideTimestamp?: boolean;
  /** Mid-turn accordion: fold into one-line preview when superseded. */
  collapsed?: boolean;
}) {
  const t = useT();
  const locale = useLocale();
  const role = blockStyle(block.block_type);
  const displayBody =
    block.block_type === "assistant_message"
      ? sanitizeAssistantDisplay(block.body, locale)
      : block.body;
  const missing =
    block.block_type === "system_notice" &&
    block.meta?.source === "missing_turn";
  const hasVisibleBody = displayBody.trim().length > 0;
  const isError = role === "error" || looksLikeError(block.body);
  const isLive = Boolean(block.meta?.live);
  const isAssistant = block.block_type === "assistant_message";
  const isStatus = isStatusMessage(block);
  const isFinalReply = isAssistant && !isStatus;
  const streamActive = isAssistant && (isLive || showStreamCursor);
  const hookTarget = hasVisibleBody && !missing ? displayBody : "";
  const hookStreamActive =
    (streamActive || forceStreamSmooth) && hasVisibleBody && !missing;
  const { text: smoothedBody, isRevealing } = useSmoothText(
    block.id,
    hookTarget,
    hookStreamActive,
  );
  const collapseStats = useContentCollapse(hookTarget);
  const [userExpanded, setUserExpanded] = useState(false);

  useEffect(() => {
    if (!collapsed) setUserExpanded(false);
  }, [collapsed]);

  if (missing) {
    return (
      <div className="rounded-xl border border-dashed border-outline-variant px-4 py-3 text-sm text-secondary italic">
        {t("conversations.noReplyRecorded")}
      </div>
    );
  }

  // Drop any reply block that has no visible content (empty assistant text,
  // blank system_notice / session_error placeholders). Prevents empty pills.
  if (!hasVisibleBody) {
    return null;
  }

  const visuallyStreaming = streamActive || isRevealing;
  const showCursor =
    showStreamCursor && visuallyStreaming && isFinalReply;

  if (collapsed && !userExpanded && isAssistant) {
    const preview = displayBody.replace(/\s+/g, " ").trim();
    return (
      <button
        type="button"
        className="agent-narration-fold"
        onClick={() => setUserExpanded(true)}
        aria-expanded={false}
        aria-label={t("common.expand")}
      >
        <Icon name="chevron_right" size={18} className="transcript-expand-icon" />
        <span className="agent-narration-fold__text">{preview}</span>
      </button>
    );
  }

  if (isStatus) {
    return (
      <div className="agent-status-line">
        <TranscriptMarkdown text={smoothedBody} live={visuallyStreaming} />
        {showCursor && <span className="chat-stream-cursor" aria-hidden />}
      </div>
    );
  }

  const shouldCollapse =
    !isAssistant &&
    !isLive &&
    (collapseStats.lines > 20 || collapseStats.chars > 1200);
  const lines = collapseStats.lines;
  const usePanel =
    !isLive &&
    (block.block_type === "system_notice" ||
      (!isAssistant &&
        (shouldCollapse || block.collapsible || block.default_collapsed)));
  const defaultOpen =
    isError ||
    (block.default_collapsed === true
      ? false
      : block.default_collapsed === false
        ? true
        : !shouldCollapse && block.block_type !== "system_notice");

  const headerActions = (
    <>
      {block.event_id && (
        <Link
          to="/events/$eventId"
          params={{ eventId: block.event_id }}
          className="dw-btn-ghost text-[10px] py-0.5 no-underline"
        >
          <Icon name="link" size={12} />
        </Link>
      )}
      {isAssistant && !isError && <SpeakButton text={displayBody} />}
      <CopyButton text={block.body} label={t("conversations.copyMessage")} />
    </>
  );

  if (usePanel && !isError) {
    const title = formatTranscriptBlockTitle(block, t);
    const subtitle = previewLines(displayBody, 2, 200);
    const meta =
      lines > 0
        ? t("conversations.messageMeta").replace("{lines}", String(lines))
        : undefined;
    return (
      <CollapsiblePanel
        title={title}
        subtitle={subtitle ? `${meta ?? ""} · ${subtitle}` : meta}
        defaultOpen={defaultOpen}
        tone={block.block_type === "system_notice" ? "muted" : "default"}
        icon="smart_toy"
        headerActions={headerActions}
      >
        <TranscriptMarkdown text={smoothedBody} live={visuallyStreaming && isAssistant} />
        {showCursor && <span className="chat-stream-cursor" aria-hidden />}
        {!hideTimestamp && (
          <time className="block mt-2 text-[11px] text-secondary">
            {formatRelativeTime(block.at)}
          </time>
        )}
      </CollapsiblePanel>
    );
  }

  if (isFinalReply && isTaskSummaryReceipt(displayBody)) {
    return (
      <div className="group relative max-w-[min(100%,42rem)]">
        <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1 z-10">
          {headerActions}
        </div>
        <TaskReceiptCard
          body={smoothedBody}
          live={visuallyStreaming}
          showCursor={showCursor}
        />
        {!hideTimestamp && (
          <time className="block mt-2 text-[11px] text-secondary">
            {formatRelativeTime(block.at)}
          </time>
        )}
      </div>
    );
  }

  return (
    <div
      className={`rounded-2xl px-4 py-3 text-sm group relative ${
        isError
          ? "rounded-bl-md bg-error-container/80 text-on-error-container border border-error/25"
          : visuallyStreaming
            ? "bubble-assistant bubble-assistant-live bubble-anycode-final glass-panel rounded-2xl rounded-bl-md px-4 py-3 text-sm group relative text-on-surface"
            : isFinalReply
              ? "bubble-assistant bubble-anycode-final glass-panel rounded-2xl rounded-bl-md px-4 py-3 text-sm group relative text-on-surface"
              : "bubble-assistant glass-panel rounded-2xl rounded-bl-md px-4 py-3 text-sm group relative text-on-surface"
      }`}
    >
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
        {headerActions}
      </div>
      {!shouldCollapse && isFinalReply && (
        <div className="flex flex-wrap items-center gap-2 text-xs mb-2">
          <span className="font-medium text-secondary">
            {isError ? t("common.error") : t("conversations.assistant")}
          </span>
        </div>
      )}
      {isError ? (
        <ErrorMessageBody text={block.body} />
      ) : (
        <>
          <TranscriptMarkdown text={smoothedBody} live={visuallyStreaming && isAssistant} />
          {showCursor && <span className="chat-stream-cursor" aria-hidden />}
        </>
      )}
      {!hideTimestamp && (
        <time className="block mt-2 text-[11px] text-secondary">
          {formatRelativeTime(block.at)}
        </time>
      )}
    </div>
  );
}, (prev, next) =>
  prev.block.id === next.block.id &&
  prev.block.body === next.block.body &&
  prev.block.block_type === next.block.block_type &&
  prev.block.meta?.live === next.block.meta?.live &&
  prev.block.meta?.narration === next.block.meta?.narration &&
  prev.showStreamCursor === next.showStreamCursor &&
  prev.forceStreamSmooth === next.forceStreamSmooth &&
  prev.hideTimestamp === next.hideTimestamp &&
  prev.modelName === next.modelName);

function ErrorMessageBody({ text }: { text: string }) {
  const t = useT();
  const { summary, raw } = useMemo(
    () =>
      humanizeTranscriptError(
        text,
        (field) => t("conversations.errorToolField").replace("{field}", field),
        (field) => t("conversations.errorMissingField").replace("{field}", field),
      ),
    [text, t],
  );
  const geoError = useMemo(() => isGeoProviderError(text), [text]);
  return (
    <div className="space-y-2 text-sm leading-relaxed">
      <p className="m-0 font-medium">{summary}</p>
      {geoError && (
        <p className="m-0 text-sm">
          {t("conversations.geoErrorHint")}{" "}
          <Link
            to="/settings"
            search={{ section: "model" }}
            className="text-primary font-medium no-underline hover:underline"
          >
            {t("conversations.geoErrorLink")}
          </Link>
        </p>
      )}
      {raw.length > summary.length + 20 && (
        <details className="text-xs">
          <summary className="cursor-pointer text-secondary">{t("common.details")}</summary>
          <pre className="mt-2 m-0 whitespace-pre-wrap break-words font-code opacity-90">
            {raw}
          </pre>
        </details>
      )}
    </div>
  );
}

/** Seconds since the transcript / live log last changed while running. */
function useStalledSeconds(isRunning: boolean, dataSignature: string): number {
  const lastActivityRef = useRef(Date.now());
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    lastActivityRef.current = Date.now();
  }, [dataSignature]);
  useEffect(() => {
    if (!isRunning) return;
    const id = setInterval(() => setNow(Date.now()), 5_000);
    return () => clearInterval(id);
  }, [isRunning]);
  if (!isRunning) return 0;
  return Math.max(0, Math.floor((now - lastActivityRef.current) / 1000));
}

function TypingIndicator({ compact }: { compact?: boolean }) {
  const t = useT();
  return (
    <div
      className={`typing-indicator rounded-2xl rounded-bl-md border border-outline-variant/80 bg-surface-container-low ${
        compact ? "px-3 py-2" : "px-4 py-3"
      }`}
      data-testid="typing-indicator"
    >
      <div className="flex items-center gap-2 text-sm text-secondary">
        <span className="inline-flex gap-1">
          <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
          <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse [animation-delay:120ms]" />
          <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse [animation-delay:240ms]" />
        </span>
        <span>{compact ? t("conversations.waitingForModel") : t("conversations.agentWorking")}</span>
      </div>
    </div>
  );
}

function ExecutionLogLink({ sessionId }: { sessionId: string }) {
  const t = useT();
  return (
    <div className="pt-1">
      <Link
        to="/sessions/$sessionId"
        params={{ sessionId }}
        search={sessionDetailSearch("debug")}
        className="text-xs text-secondary no-underline inline-flex items-center gap-1 hover:text-primary"
      >
        <Icon name="timeline" size={14} />
        {t("conversations.viewExecutionLog")}
      </Link>
    </div>
  );
}

function blocksToTurns(blocks: TranscriptBlock[]): ConversationTurn[] {
  const turns: ConversationTurn[] = [];
  let current: ConversationTurn | null = null;

  for (const block of blocks) {
    if (block.block_type === "user_message") {
      if (current) turns.push(current);
      current = { id: block.id, user: block, replies: [] };
      continue;
    }
    if (!current) continue;
    if (isReplyBlock(block.block_type)) {
      current.replies.push(block);
    }
  }
  if (current) turns.push(current);
  return turns;
}

function isReplyBlock(blockType: string): boolean {
  return [
    "assistant_message",
    "session_error",
    "tool_call",
    "tool_result",
    "system_notice",
    "deliverable",
  ].includes(blockType);
}

function deliverablePropsFromBlock(block: TranscriptBlock): DeliverableCardProps | null {
  const meta = block.meta ?? {};
  const path = typeof meta.path === "string" ? meta.path.trim() : "";
  if (!path) return null;
  const bytesRaw = meta.bytes;
  return {
    path,
    title: typeof meta.title === "string" ? meta.title : block.title || undefined,
    kind: typeof meta.kind === "string" ? meta.kind : undefined,
    mime: typeof meta.mime === "string" ? meta.mime : undefined,
    projectId: typeof meta.project_id === "string" ? meta.project_id : undefined,
    previewPath: typeof meta.preview_path === "string" ? meta.preview_path : undefined,
    bytes: typeof bytesRaw === "number" ? bytesRaw : undefined,
  };
}

function isGeoProviderError(text: string): boolean {
  const lower = text.toLowerCase();
  return (
    lower.includes("user location is not supported") ||
    lower.includes("user location") ||
    (lower.includes("failed_precondition") && lower.includes("location"))
  );
}

function looksLikeError(text: string): boolean {
  const lower = text.toLowerCase();
  return (
    lower.includes("llm error") ||
    lower.includes("api error") ||
    lower.includes("failed_precondition") ||
    lower.includes("status=400")
  );
}

function blockStyle(blockType: string): "user" | "assistant" | "error" | "system" {
  if (blockType === "user_message") return "user";
  if (blockType === "assistant_message") return "assistant";
  if (blockType === "session_error") return "error";
  return "system";
}
