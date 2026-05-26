import { useEffect, useMemo, useRef } from "react";
import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { formatRelativeTime } from "@/utils/formatTime";
import { useT } from "@/i18n/context";

interface Props {
  sessionId: string | null;
  enabled?: boolean;
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

export function ConversationTranscript({
  sessionId,
  enabled = true,
  isRunning,
  scrollContainerRef,
}: Props) {
  const t = useT();
  const bottomRef = useRef<HTMLDivElement>(null);

  const transcript = useQuery({
    queryKey: ["session-transcript", sessionId],
    queryFn: () => api.sessionTranscript(sessionId!),
    enabled: Boolean(sessionId) && enabled,
    refetchInterval: isRunning ? 1_000 : 3_000,
  });

  const liveLog = useQuery({
    queryKey: ["session-execution-log-live", sessionId],
    queryFn: () => api.sessionExecutionLog(sessionId!, { offset: 0, limit: 120 }),
    enabled: Boolean(sessionId) && enabled && Boolean(isRunning),
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

  useEffect(() => {
    const container = scrollContainerRef?.current;
    if (container) {
      container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
      return;
    }
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }, [turns.length, blocks.length, isRunning, activeTool, scrollContainerRef]);

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
    <article className="flex flex-col gap-3">
      <MessageRow align="right">
        <UserBubble block={turn.user} />
      </MessageRow>

      {replyItems.map((item) => {
        if (item.kind === "tool_group") {
          return (
            <MessageRow key={item.id} align="left">
              <ToolGroupCard tools={item.tools} />
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
  return (
    <div className="rounded-2xl rounded-br-md bg-primary text-on-primary px-4 py-3 text-sm shadow-sm">
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

  return (
    <div
      className={`rounded-2xl px-4 py-3 text-sm shadow-sm ${
        isError
          ? "rounded-bl-md bg-error-container/80 text-on-error-container border border-error/25"
          : "rounded-bl-md bg-surface-container-low text-on-surface border border-outline-variant/70"
      }`}
    >
      <div className="text-xs font-medium text-secondary mb-2">
        {isError ? t("common.error") : t("conversations.assistant")}
      </div>
      {isError ? (
        <ErrorMessageBody text={block.body} />
      ) : (
        <AssistantMessageBody text={block.body} />
      )}
      <time className="block mt-2 text-[11px] text-secondary">
        {formatRelativeTime(block.at)}
      </time>
    </div>
  );
}

function ErrorMessageBody({ text }: { text: string }) {
  const summary = useMemo(() => summarizeError(text), [text]);
  return (
    <div className="space-y-2 text-sm leading-relaxed">
      <p className="m-0 font-medium">{summary}</p>
      {text.length > summary.length + 20 && (
        <details className="text-xs">
          <summary className="cursor-pointer text-secondary">Details</summary>
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
        {!compact && (
          <span>{t("conversations.agentWorking")}</span>
        )}
      </div>
    </div>
  );
}

function LiveToolCard({ toolName }: { toolName: string }) {
  return (
    <div className="rounded-2xl rounded-bl-md border border-primary/25 bg-primary-container/20 px-4 py-3 text-sm">
      <div className="flex items-center gap-2 text-primary font-medium">
        <span className="inline-block w-2 h-2 rounded-full bg-primary animate-pulse" />
        <Icon name="build" size={16} />
        <span>{toolName}</span>
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

function ToolGroupCard({ tools }: { tools: TranscriptBlock[] }) {
  const t = useT();
  const title =
    tools.find((b) => b.block_type === "tool_call")?.title ||
    tools[0]?.title ||
    t("conversations.toolActivity");
  const failed = tools.some(
    (tool) =>
      tool.block_type === "tool_result" &&
      /failed|error|denied/i.test(`${tool.title} ${tool.body}`),
  );
  const running =
    tools.some((tool) => tool.block_type === "tool_call") &&
    !tools.some((tool) => tool.block_type === "tool_result");

  return (
    <div
      className={`rounded-2xl rounded-bl-md border overflow-hidden w-full ${
        failed
          ? "border-error/30 bg-error-container/15"
          : running
            ? "border-primary/25 bg-primary-container/15"
            : "border-outline-variant bg-surface-container-low"
      }`}
    >
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-outline-variant/60">
        {running && (
          <span className="inline-block w-2 h-2 rounded-full bg-primary animate-pulse shrink-0" />
        )}
        <Icon name="build" size={16} className="text-secondary shrink-0" />
        <span className="text-sm font-medium truncate">{title}</span>
        <span className="text-[10px] text-secondary shrink-0">
          {tools.length} {t("conversations.toolSteps")}
        </span>
      </div>
      <div className="divide-y divide-outline-variant/60">
        {tools.map((tool) => (
          <div key={tool.id} className="px-4 py-2.5 text-sm">
            {tool.body ? (
              <pre className="m-0 whitespace-pre-wrap break-words font-code text-xs text-on-surface">
                {tool.body}
              </pre>
            ) : (
              <span className="text-xs text-secondary">{tool.title}</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function AssistantMessageBody({ text }: { text: string }) {
  const parts = useMemo(() => splitMarkdownLike(text), [text]);
  return (
    <div className="space-y-2 text-sm leading-relaxed">
      {parts.map((part, idx) =>
        part.kind === "code" ? (
          <pre
            key={idx}
            className="m-0 overflow-x-auto rounded-lg bg-surface-container-highest px-3 py-2 font-code text-xs whitespace-pre-wrap break-words"
          >
            {part.text}
          </pre>
        ) : (
          <div key={idx} className="whitespace-pre-wrap break-words">
            {part.text}
          </div>
        ),
      )}
    </div>
  );
}

function splitMarkdownLike(text: string): { kind: "text" | "code"; text: string }[] {
  const parts: { kind: "text" | "code"; text: string }[] = [];
  const re = /```[^\n]*\n([\s\S]*?)```/g;
  let last = 0;
  for (const match of text.matchAll(re)) {
    const start = match.index ?? 0;
    if (start > last) {
      parts.push({ kind: "text", text: text.slice(last, start).trim() });
    }
    parts.push({ kind: "code", text: (match[1] ?? "").trim() });
    last = start + match[0].length;
  }
  if (last < text.length) {
    parts.push({ kind: "text", text: text.slice(last).trim() });
  }
  if (parts.length === 0) {
    parts.push({ kind: "text", text });
  }
  return parts.filter((p) => p.text.length > 0);
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
