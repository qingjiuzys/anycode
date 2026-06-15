import { useCallback, useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventSource } from "@/hooks/useEventSource";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";
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

/** Per-session SSE; returns whether the stream is live. */
export function useSessionEventStream(
  sessionId: string | undefined,
  scope: SessionStreamScope = "conversation",
): boolean {
  const queryClient = useQueryClient();
  const heavyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (heavyTimer.current) clearTimeout(heavyTimer.current);
    };
  }, []);

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

  const onEvent = useCallback(
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
        scheduleHeavy(heavyKeys);
        return;
      }

      scheduleHeavy(heavyKeys);
    },
    [heavyKeysForScope, invalidateNow, queryClient, scheduleHeavy, sessionId],
  );

  const status = useEventSource(
    sessionId
      ? `${API_BASE}/api/sessions/${sessionId}/events/stream`
      : null,
    onEvent,
  );

  return status === "live" || status === "connecting";
}
