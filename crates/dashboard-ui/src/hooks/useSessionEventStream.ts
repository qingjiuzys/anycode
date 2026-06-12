import { useCallback, useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventSource } from "@/hooks/useEventSource";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";
const HEAVY_INVALIDATE_MS = 4_000;

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

  const scheduleHeavy = useCallback(
    (keys: readonly (readonly unknown[])[]) => {
      if (heavyTimer.current) clearTimeout(heavyTimer.current);
      heavyTimer.current = setTimeout(() => {
        if (!sessionId) return;
        for (const queryKey of keys) {
          void queryClient.invalidateQueries({ queryKey });
        }
      }, HEAVY_INVALIDATE_MS);
    },
    [queryClient, sessionId],
  );

  const onEvent = useCallback(() => {
    if (!sessionId) {
      return;
    }

    const heavyKeys: (readonly unknown[])[] =
      scope === "detail"
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
            ["session-workflow-events", sessionId],
            ["session-plan-events", sessionId],
          ]
        : [
            ["session", sessionId],
            ["session-transcript", sessionId],
            ["session-execution-log-live", sessionId],
            ["session-artifacts", sessionId],
          ];

    scheduleHeavy(heavyKeys);
  }, [queryClient, scheduleHeavy, scope, sessionId]);

  const status = useEventSource(
    sessionId
      ? `${API_BASE}/api/sessions/${sessionId}/events/stream`
      : null,
    onEvent,
  );

  return status === "live" || status === "connecting";
}
