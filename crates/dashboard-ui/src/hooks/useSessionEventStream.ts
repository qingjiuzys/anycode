import { useCallback, useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { TranscriptBlock } from "@/api/types";
import { useEventSource, type SseStatus } from "@/hooks/useEventSource";
import {
  applyChatStreamEvent,
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
  liveBlocks: TranscriptBlock[];
  status: SseStatus;
};

export type SessionEventStreamOptions = {
  onTurnDone?: () => void;
};

/** Per-session SSE: single EventSource for project_event + chat_event. */
export function useSessionEventStream(
  sessionId: string | undefined,
  scope: SessionStreamScope = "conversation",
  options: SessionEventStreamOptions = {},
): SessionEventStreamState {
  const queryClient = useQueryClient();
  const onTurnDone = options.onTurnDone;
  const heavyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [liveBlocks, setLiveBlocks] = useState<TranscriptBlock[]>([]);
  const [chatLive, setChatLive] = useState(false);
  const blocksRef = useRef<TranscriptBlock[]>([]);

  useEffect(() => {
    return () => {
      if (heavyTimer.current) clearTimeout(heavyTimer.current);
    };
  }, []);

  const resetLive = useCallback(() => {
    blocksRef.current = [];
    setLiveBlocks([]);
  }, []);

  useEffect(() => {
    resetLive();
    setChatLive(false);
  }, [resetLive, sessionId]);

  const applyChatEvent = useCallback(
    (evt: ChatStreamEvent) => {
      if (evt.kind === "turn_done") {
        resetLive();
        setChatLive(false);
        onTurnDone?.();
        if (sessionId) {
          void queryClient.invalidateQueries({
            queryKey: ["session-transcript", sessionId],
          });
          void queryClient.invalidateQueries({
            queryKey: ["session", sessionId],
          });
        }
        return;
      }
      setChatLive(true);
      blocksRef.current = applyChatStreamEvent(blocksRef.current, evt);
      setLiveBlocks([...blocksRef.current]);
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

  const status = useEventSource(
    sessionId
      ? apiUrl(`/api/sessions/${sessionId}/events/stream`)
      : null,
    onProjectEvent,
    scope === "conversation" ? onChatEvent : undefined,
  );

  const connected = status === "live" || status === "connecting";

  return {
    connected,
    live: chatLive || connected,
    liveBlocks,
    status,
  };
}
