import { useEffect, useMemo, useRef } from "react";
import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api } from "@/api/client";
import type { TranscriptBlock } from "@/api/types";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/Icon";
import {
  isCommandBlock,
  TranscriptCommandBlock,
} from "@/components/TranscriptCommandBlock";
import { TranscriptMarkdown } from "@/components/TranscriptMarkdown";
import { TranscriptToolBlock } from "@/components/TranscriptToolBlock";
import {
  CollapsiblePanel,
  previewLines,
  useContentCollapse,
} from "@/components/ui/CollapsiblePanel";
import { formatRelativeTime } from "@/utils/formatTime";
import { formatLiveToolLabel, formatTranscriptBlockTitle } from "@/lib/eventFormat";
import { useT } from "@/i18n/context";

interface Props {
  sessionId: string | null;
  isRunning?: boolean;
  scrollContainerRef?: React.RefObject<HTMLElement | null>;
}

type ConversationTurn = {
  id: string;
  user: TranscriptBlock;
  replies: TranscriptBlock[];
};

type ReplyItem =
  | { kind: "block"; block: TranscriptBlock }
  | { kind: "tool_group"; id: string; tools: TranscriptBlock[] };

const VIRTUAL_TURN_THRESHOLD = 30;

export function ConversationTranscript({
  sessionId,
  isRunning,
  scrollContainerRef,
}: Props) {
  const t = useT();
  const bottomRef = useRef<HTMLDivElement>(null);
  const localScrollRef = useRef<HTMLDivElement>(null);

  const transcript = useQuery({
    queryKey: ["session-transcript", sessionId],
    queryFn: () => api.sessionTranscript(sessionId!),
    enabled: Boolean(sessionId),
    refetchInterval: isRunning ? 1_000 : 3_000,
  });

  const liveLog = useQuery({
    queryKey: ["session-execution-log-live", sessionId],
    queryFn: () => api.sessionExecutionLog(sessionId!, { offset: 0, limit: 120 }),
    enabled: Boolean(sessionId) && Boolean(isRunning),
    refetchInterval: 1_000,
  });

  const blocks = transcript.data?.transcript.blocks ?? [];
  const lifecycleCount = transcript.data?.transcript.lifecycle?.length ?? 0;
  const turns = useMemo(() => blocksToTurns(blocks), [blocks]);
  const activeTool = useMemo(
    () => findActiveTool(liveLog.data?.execution_log.lines ?? []),
    [liveLog.data?.execution_log.lines],
  );
  const lastTurnPending =
    isRunning &&
    turns.length > 0 &&
    turns[turns.length - 1].replies.length === 0;

  const useVirtual = turns.length >= VIRTUAL_TURN_THRESHOLD;
  const virtualParentRef = scrollContainerRef ?? localScrollRef;

  const virtualizer = useVirtualizer({
    count: turns.length,
    getScrollElement: () => virtualParentRef.current,
    estimateSize: () => 220,
    overscan: 4,
  });

  useEffect(() => {
    if (useVirtual) {
      virtualizer.scrollToIndex(turns.length - 1, { align: "end", behavior: "smooth" });
      return;
    }
    const container = scrollContainerRef?.current;
    if (container) {
      container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
      return;
    }
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }, [
    turns.length,
    blocks.length,
    isRunning,
    activeTool,
    scrollContainerRef,
    useVirtual,
    virtualizer,
  ]);

  if (!sessionId) return null;
  if (transcript.isLoading) {
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
      {lastTurnPending && (
        <div className="flex w-full justify-start">
          <div className="max-w-[min(100%,42rem)] w-full">
            {activeTool ? (
              <LiveToolCard toolName={activeTool} />
            ) : (
              <TypingIndicator />
            )}
          </div>
        </div>
      )}
      {lifecycleCount > 0 && <ExecutionLogLink sessionId={sessionId} />}
      <div ref={bottomRef} aria-hidden className="h-px shrink-0" />
    </>
  );

  if (useVirtual) {
    return (
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
              />
            </div>
          );
        })}
        <div className="flex flex-col gap-8 pt-4">{tail}</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8">
      {turns.map((turn, index) => (
        <ConversationTurnView
          key={turn.id}
          turn={turn}
          isLast={index === turns.length - 1}
          isRunning={Boolean(isRunning)}
        />
      ))}
      {tail}
    </div>
  );
}

function ConversationTurnView({
  turn,
  isLast,
  isRunning,
}: {
  turn: ConversationTurn;
  isLast: boolean;
  isRunning: boolean;
}) {
  const replyItems = useMemo(() => groupToolBlocks(turn.replies), [turn.replies]);

  return (
    <article className="flex flex-col gap-3 pb-8">
      <MessageRow align="right">
        <UserBubble block={turn.user} />
      </MessageRow>

      {replyItems.map((item) => {
        if (item.kind === "tool_group") {
          return (
            <MessageRow key={item.id} align="left">
              <TranscriptToolBlock
                tools={item.tools}
                defaultCollapsed={
                  !isRunning ||
                  !item.tools.some(
                    (tool) =>
                      tool.block_type === "tool_call" && !tool.default_collapsed,
                  )
                }
              />
            </MessageRow>
          );
        }
        if (isCommandBlock(item.block)) {
          return (
            <MessageRow key={item.block.id} align="left">
              <TranscriptCommandBlock block={item.block} />
            </MessageRow>
          );
        }
        return (
          <MessageRow key={item.block.id} align="left">
            <ReplyBubble block={item.block} />
          </MessageRow>
        );
      })}

      {isLast && isRunning && turn.replies.length > 0 && (
        <MessageRow align="left">
          <TypingIndicator compact />
        </MessageRow>
      )}
    </article>
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
      <div className="max-w-[min(100%,42rem)] w-fit min-w-0">{children}</div>
    </div>
  );
}

function UserBubble({ block }: { block: TranscriptBlock }) {
  const t = useT();
  return (
    <div className="rounded-2xl rounded-br-md bg-primary text-on-primary px-4 py-3 text-sm shadow-sm group relative">
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
        <CopyButton text={block.body} label={t("conversations.copyMessage")} />
      </div>
      <div className="whitespace-pre-wrap break-words leading-relaxed">{block.body}</div>
      <time className="block mt-2 text-[11px] opacity-70">{formatRelativeTime(block.at)}</time>
    </div>
  );
}

function ReplyBubble({ block }: { block: TranscriptBlock }) {
  const t = useT();
  const role = blockStyle(block.block_type);
  const missing =
    block.block_type === "system_notice" &&
    block.meta?.source === "missing_turn";

  if (missing) {
    return (
      <div className="rounded-xl border border-dashed border-outline-variant px-4 py-3 text-sm text-secondary italic">
        {t("conversations.noReplyRecorded")}
      </div>
    );
  }

  const isError = role === "error" || looksLikeError(block.body);
  const { shouldCollapse, lines } = useContentCollapse(block.body);
  const usePanel =
    shouldCollapse ||
    block.collapsible ||
    block.default_collapsed ||
    block.block_type === "system_notice";
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
      <CopyButton text={block.body} label={t("conversations.copyMessage")} />
    </>
  );

  if (usePanel && !isError) {
    const title = formatTranscriptBlockTitle(block, t);
    const subtitle = previewLines(block.body, 2, 200);
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
        <TranscriptMarkdown text={block.body} />
        <time className="block mt-2 text-[11px] text-secondary">
          {formatRelativeTime(block.at)}
        </time>
      </CollapsiblePanel>
    );
  }

  return (
    <div
      className={`rounded-2xl px-4 py-3 text-sm group relative ${
        isError
          ? "rounded-bl-md bg-error-container/80 text-on-error-container border border-error/25"
          : "rounded-bl-md bg-surface-container-lowest text-on-surface border border-outline-variant/60"
      }`}
    >
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
        {headerActions}
      </div>
      {!shouldCollapse && (
        <div className="text-xs font-medium text-secondary mb-2">
          {isError ? t("common.error") : t("conversations.assistant")}
        </div>
      )}
      {isError ? (
        <ErrorMessageBody text={block.body} />
      ) : (
        <TranscriptMarkdown text={block.body} />
      )}
      <time className="block mt-2 text-[11px] text-secondary">
        {formatRelativeTime(block.at)}
      </time>
    </div>
  );
}

function ErrorMessageBody({ text }: { text: string }) {
  const t = useT();
  const summary = useMemo(() => summarizeError(text), [text]);
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
      {text.length > summary.length + 20 && (
        <details className="text-xs">
          <summary className="cursor-pointer text-secondary">{t("common.details")}</summary>
          <pre className="mt-2 m-0 whitespace-pre-wrap break-words font-code opacity-90">
            {text}
          </pre>
        </details>
      )}
    </div>
  );
}

function summarizeError(text: string): string {
  const first = text.split("\n").find((line) => line.trim())?.trim() ?? text.trim();
  if (first.length > 220) {
    return `${first.slice(0, 220)}…`;
  }
  return first;
}

function TypingIndicator({ compact }: { compact?: boolean }) {
  const t = useT();
  return (
    <div
      className={`rounded-2xl rounded-bl-md border border-outline-variant/80 bg-surface-container-low ${
        compact ? "px-3 py-2" : "px-4 py-3"
      }`}
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

function LiveToolCard({ toolName }: { toolName: string }) {
  const t = useT();
  return (
    <div className="rounded-2xl rounded-bl-md border border-primary/25 bg-primary-container/20 px-4 py-3 text-sm">
      <div className="flex items-center gap-2 text-primary font-medium">
        <span className="inline-block w-2 h-2 rounded-full bg-primary animate-pulse" />
        <Icon name="build" size={16} />
        <span>{formatLiveToolLabel(toolName, t)}</span>
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
  ].includes(blockType);
}

function groupToolBlocks(blocks: TranscriptBlock[]): ReplyItem[] {
  const out: ReplyItem[] = [];
  let toolBatch: TranscriptBlock[] = [];
  const flushTools = () => {
    if (toolBatch.length === 0) return;
    out.push({
      kind: "tool_group",
      id: `tools:${toolBatch[0]?.id ?? "batch"}`,
      tools: toolBatch,
    });
    toolBatch = [];
  };
  for (const block of blocks) {
    if (block.block_type === "tool_call" || block.block_type === "tool_result") {
      toolBatch.push(block);
      continue;
    }
    flushTools();
    out.push({ kind: "block", block });
  }
  flushTools();
  return out;
}

function findActiveTool(
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
