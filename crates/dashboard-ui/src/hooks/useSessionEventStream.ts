import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { TranscriptBlock } from "@/api/types";
import { useEventSource, type SseStatus } from "@/hooks/useEventSource";
import {
  applyChatStreamEvent,
  blocksFromCanonicalEvents,
  type ChatStreamEvent,
} from "@/lib/liveTranscript";
import { apiUrl } from "@/api/http";

const HEAVY_INVALIDATE_MS = 2_000;

const IMMEDIATE_EVENT_TYPES = new Set([
  "user_prompt",
  "prompt",
  "assistant_response",
  "task_end",
  "session_completed",
  "session_blocked",
  "session_error",
  "session_cancelled",
]);

const LIGHT_EVENT_TYPES = new Set([
  "tool_call_start",
  "tool_call_end",
  "tool_call_input",
  "tool_denied",
  "tool_approval_pending",
  "tool_approval_resolved",
]);

export type SessionStreamScope = "conversation" | "detail";

export type SessionEventStreamState = {
  /** SSE transport connected (connecting or live). */
  connected: boolean;
  /** Assistant/tool chat stream is active. */
  live: boolean;
  liveEvents: ChatStreamEvent[];
  liveBlocks: TranscriptBlock[];
  status: SseStatus;
};

export type SessionEventStreamOptions = {
  onTurnDone?: () => void;
  /** Current session status from list/detail (drives replay vs live handling). */
  sessionStatus?: string;
  /** True while waiting for backend to mark session running after send. */
  optimisticStreaming?: boolean;
};

/** True when SSE recovered from a drop and should reconnect with after_seq replay. */
export function shouldRebaseLiveOnSseReconnect(
  prev: SseStatus,
  next: SseStatus,
): boolean {
  return next === "live" && prev === "reconnecting";
}

/** Whether incoming chat_event payloads should populate live transcript state. */
export function shouldTrackChatEventAsLive(
  sessionStatus: string | undefined,
  optimisticStreaming: boolean,
): boolean {
  if (optimisticStreaming) {
    return true;
  }
  return sessionStatus === "running";
}

/** streamLive for canonical merge / polling (not the same as SSE connected). */
export function conversationStreamLive(
  chatStreamLive: boolean,
  sseLive: boolean,
  running: boolean,
): boolean {
  return chatStreamLive || (sseLive && running);
}

/** running flag for conversation thread UI. */
export function conversationThreadRunning(
  sessionStatus: string,
  sessionId: string,
  optimisticStreamingSessionId: string | null,
): boolean {
  return (
    sessionStatus === "running" || optimisticStreamingSessionId === sessionId
  );
}

function invalidateSessionListQueries(
  queryClient: ReturnType<typeof useQueryClient>,
): void {
  void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
  void queryClient.invalidateQueries({ queryKey: ["session-facets"] });
}

/** Per-session SSE: single EventSource for project_event + chat_event replay. */
export function useSessionEventStream(
  sessionId: string | undefined,
  scope: SessionStreamScope = "conversation",
  options: SessionEventStreamOptions = {},
): SessionEventStreamState {
  const queryClient = useQueryClient();
  const onTurnDone = options.onTurnDone;
  const sessionStatus = options.sessionStatus;
  const optimisticStreaming = options.optimisticStreaming ?? false;
  const heavyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [liveEvents, setLiveEvents] = useState<ChatStreamEvent[]>([]);
  const [chatLive, setChatLive] = useState(false);
  const eventsRef = useRef<Map<number, ChatStreamEvent>>(new Map());
  const lastSeqRef = useRef(0);
  const [afterSeq, setAfterSeq] = useState(0);
  const prevStatusRef = useRef<SseStatus>("offline");
  const trackLiveRef = useRef(
    shouldTrackChatEventAsLive(sessionStatus, optimisticStreaming),
  );

  trackLiveRef.current = shouldTrackChatEventAsLive(
    sessionStatus,
    optimisticStreaming,
  );

  useEffect(() => {
    return () => {
      if (heavyTimer.current) clearTimeout(heavyTimer.current);
    };
  }, []);

  const resetLive = useCallback(() => {
    eventsRef.current = new Map();
    setLiveEvents([]);
  }, []);

  useEffect(() => {
    resetLive();
    setChatLive(false);
    setAfterSeq(0);
    lastSeqRef.current = 0;
  }, [resetLive, sessionId]);

  const applyChatEvent = useCallback(
    (evt: ChatStreamEvent) => {
      const trackLive = trackLiveRef.current;

      if (evt.kind === "turn_done") {
        setChatLive(false);
        onTurnDone?.();
        if (evt.seq !== undefined) {
          lastSeqRef.current = Math.max(lastSeqRef.current, evt.seq);
        }
        resetLive();
        if (sessionId) {
          void queryClient.invalidateQueries({
            queryKey: ["session-transcript", sessionId],
          });
          void queryClient.invalidateQueries({
            queryKey: ["session", sessionId],
          });
          invalidateSessionListQueries(queryClient);
        }
        return;
      }

      if (evt.kind === "session_error") {
        setChatLive(false);
        resetLive();
        if (evt.seq !== undefined) {
          lastSeqRef.current = Math.max(lastSeqRef.current, evt.seq);
        }
        if (sessionId) {
          void queryClient.invalidateQueries({
            queryKey: ["session-transcript", sessionId],
          });
          void queryClient.invalidateQueries({
            queryKey: ["session", sessionId],
          });
          invalidateSessionListQueries(queryClient);
        }
        return;
      }

      if (evt.seq !== undefined) {
        if (evt.seq <= lastSeqRef.current) {
          return;
        }
        lastSeqRef.current = Math.max(lastSeqRef.current, evt.seq);
        if (!trackLive) {
          return;
        }
      } else if (!trackLive) {
        return;
      }

      setChatLive(true);
      if (evt.seq !== undefined) {
        eventsRef.current.set(evt.seq, evt);
        setLiveEvents(
          [...eventsRef.current.values()].sort(
            (a, b) => (a.seq ?? 0) - (b.seq ?? 0),
          ),
        );
      } else {
        setLiveEvents((prev) => [...prev, evt]);
      }
      if (evt.kind === "assistant_done" && sessionId) {
        void queryClient.invalidateQueries({
          queryKey: ["session-transcript", sessionId],
        });
      }
    },
    [onTurnDone, queryClient, resetLive, sessionId],
  );

  const heavyKeysForScope = useCallback((): (readonly unknown[])[] => {
    if (!sessionId) {
      return [];
    }
    return scope === "detail"
      ? [
          ["session", sessionId],
          ["session-events", sessionId],
          ["session-gates", sessionId],
          ["session-event-types", sessionId],
          ["session-transcript", sessionId],
          ["session-execution-log-live", sessionId],
          ["session-artifacts", sessionId],
          ["session-replay", sessionId],
          ["session-trace-progress", sessionId],
          ["session-trace-inspector", sessionId],
          ["session-workflow-events", sessionId],
          ["session-plan-events", sessionId],
        ]
      : [
          ["session", sessionId],
          ["session-transcript", sessionId],
          ["session-execution-log-live", sessionId],
          ["session-artifacts", sessionId],
          ["session-trace-inspector", sessionId],
        ];
  }, [scope, sessionId]);

  const invalidateNow = useCallback(
    (keys: (readonly unknown[])[]) => {
      if (!sessionId) return;
      for (const queryKey of keys) {
        void queryClient.invalidateQueries({ queryKey });
      }
    },
    [queryClient, sessionId],
  );

  const scheduleHeavy = useCallback(
    (keys: readonly (readonly unknown[])[]) => {
      if (heavyTimer.current) clearTimeout(heavyTimer.current);
      heavyTimer.current = setTimeout(() => {
        invalidateNow([...keys]);
      }, HEAVY_INVALIDATE_MS);
    },
    [invalidateNow],
  );

  const onProjectEvent = useCallback(
    (payload: { eventType?: string }) => {
      if (!sessionId) {
        return;
      }

      const heavyKeys = heavyKeysForScope();
      const eventType = payload.eventType?.trim().toLowerCase() ?? "";

      if (IMMEDIATE_EVENT_TYPES.has(eventType)) {
        if (heavyTimer.current) clearTimeout(heavyTimer.current);
        invalidateNow(heavyKeys);
        invalidateSessionListQueries(queryClient);
        return;
      }

      if (LIGHT_EVENT_TYPES.has(eventType)) {
        void queryClient.invalidateQueries({
          queryKey: ["session-execution-log-live", sessionId],
        });
        void queryClient.invalidateQueries({
          queryKey: ["session-trace-progress", sessionId],
        });
        void queryClient.invalidateQueries({
          queryKey: ["session-trace-inspector", sessionId],
        });
        void queryClient.invalidateQueries({
          queryKey: ["session-transcript", sessionId],
        });
        scheduleHeavy(heavyKeys.filter((k) => k[0] !== "session-transcript"));
        return;
      }

      scheduleHeavy(heavyKeys);
    },
    [heavyKeysForScope, invalidateNow, queryClient, scheduleHeavy, sessionId],
  );

  const onChatEvent = useCallback(
    (raw: MessageEvent) => {
      if (!sessionId) return;
      try {
        const evt = JSON.parse(raw.data) as ChatStreamEvent;
        if (evt.session_id !== sessionId) return;
        applyChatEvent(evt);
      } catch {
        /* ignore malformed payload */
      }
    },
    [applyChatEvent, sessionId],
  );

  const onStreamReset = useCallback((payload: { last_seq?: number }) => {
    const seq = payload.last_seq ?? lastSeqRef.current;
    setAfterSeq(seq);
  }, []);

  const sseUrl = useMemo(() => {
    if (!sessionId) {
      return null;
    }
    const base = apiUrl(`/api/sessions/${sessionId}/events/stream`);
    return afterSeq > 0 ? `${base}?after_seq=${afterSeq}` : base;
  }, [afterSeq, sessionId]);

  const status = useEventSource(
    sseUrl,
    onProjectEvent,
    scope === "conversation" ? onChatEvent : undefined,
    onStreamReset,
  );

  useEffect(() => {
    const prev = prevStatusRef.current;
    prevStatusRef.current = status;
    if (shouldRebaseLiveOnSseReconnect(prev, status)) {
      setAfterSeq(lastSeqRef.current);
    }
  }, [status]);

  const liveBlocks = useMemo(() => {
    if (liveEvents.length === 0) {
      return [];
    }
    const withSeq = liveEvents.filter((evt) => evt.seq !== undefined);
    if (withSeq.length > 0) {
      return blocksFromCanonicalEvents(withSeq);
    }
    let blocks: TranscriptBlock[] = [];
    for (const evt of liveEvents) {
      blocks = applyChatStreamEvent(blocks, evt);
    }
    return blocks;
  }, [liveEvents]);

  const connected = status === "live" || status === "connecting";

  return {
    connected,
    live: chatLive,
    liveEvents,
    liveBlocks,
    status,
  };
}
