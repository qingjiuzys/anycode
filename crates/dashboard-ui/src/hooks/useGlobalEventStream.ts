import { useCallback, useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventSource, type SseStatus } from "@/hooks/useEventSource";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";
const INVALIDATION_DEBOUNCE_MS = 750;

/** Global SSE + query invalidation; returns connection status for the sidebar badge. */
export function useGlobalEventStream(enabled = true): SseStatus {
  const queryClient = useQueryClient();
  const pendingProjectIds = useRef<Set<string>>(new Set());
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flushInvalidations = useCallback(() => {
    timer.current = null;
    const projectIds = Array.from(pendingProjectIds.current);
    pendingProjectIds.current.clear();

    void queryClient.invalidateQueries({ queryKey: ["projects"] });
    void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
    void queryClient.invalidateQueries({ queryKey: ["recent-events"] });
    void queryClient.invalidateQueries({ queryKey: ["overview"] });
    void queryClient.invalidateQueries({ queryKey: ["running-sessions"] });

    for (const projectId of projectIds) {
      void queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["events", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["project-stats", projectId] });
    }
  }, [queryClient]);

  const onEvent = useCallback(
    (projectId?: string) => {
      if (projectId) {
        pendingProjectIds.current.add(projectId);
      }
      if (!timer.current) {
        timer.current = setTimeout(flushInvalidations, INVALIDATION_DEBOUNCE_MS);
      }
    },
    [flushInvalidations],
  );

  useEffect(
    () => () => {
      if (timer.current) {
        clearTimeout(timer.current);
      }
    },
    [],
  );

  return useEventSource(
    enabled ? `${API_BASE}/api/events/stream` : null,
    onEvent,
  );
}
