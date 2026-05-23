import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventSource } from "@/hooks/useEventSource";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";

/** Per-session SSE; returns whether the stream is live. */
export function useSessionEventStream(sessionId: string | undefined): boolean {
  const queryClient = useQueryClient();

  const onEvent = useCallback(() => {
    if (!sessionId) {
      return;
    }
    queryClient.invalidateQueries({ queryKey: ["session-events", sessionId] });
    queryClient.invalidateQueries({ queryKey: ["session-gates", sessionId] });
    queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    queryClient.invalidateQueries({
      queryKey: ["session-event-types", sessionId],
    });
    queryClient.invalidateQueries({
      queryKey: ["session-artifacts", sessionId],
    });
    queryClient.invalidateQueries({
      queryKey: ["session-replay", sessionId],
    });
  }, [sessionId, queryClient]);

  const status = useEventSource(
    sessionId
      ? `${API_BASE}/api/sessions/${sessionId}/events/stream`
      : null,
    onEvent,
  );

  return status === "live" || status === "connecting";
}
