import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventSource, type SseStatus } from "@/hooks/useEventSource";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";

/** Global SSE + query invalidation; returns connection status for the sidebar badge. */
export function useGlobalEventStream(enabled = true): SseStatus {
  const queryClient = useQueryClient();

  const onEvent = useCallback(
    (projectId?: string) => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      queryClient.invalidateQueries({ queryKey: ["recent-events"] });
      queryClient.invalidateQueries({ queryKey: ["automation-sessions"] });
      queryClient.invalidateQueries({ queryKey: ["cron-runs"] });
      queryClient.invalidateQueries({ queryKey: ["cron-jobs"] });
      queryClient.invalidateQueries({ queryKey: ["agent-stats"] });
      queryClient.invalidateQueries({ queryKey: ["health"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
      queryClient.invalidateQueries({ queryKey: ["running-sessions"] });
      queryClient.invalidateQueries({ queryKey: ["artifacts"] });
      queryClient.invalidateQueries({ queryKey: ["session-artifacts"] });
      queryClient.invalidateQueries({ queryKey: ["skills"] });
      queryClient.invalidateQueries({ queryKey: ["session"] });
      queryClient.invalidateQueries({ queryKey: ["session-events"] });
      queryClient.invalidateQueries({ queryKey: ["session-gates"] });
      if (projectId) {
        queryClient.invalidateQueries({ queryKey: ["project", projectId] });
        queryClient.invalidateQueries({ queryKey: ["events", projectId] });
        queryClient.invalidateQueries({
          queryKey: ["project-event-types", projectId],
        });
        queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
        queryClient.invalidateQueries({ queryKey: ["gates", projectId] });
        queryClient.invalidateQueries({
          queryKey: ["project-stats", projectId],
        });
        queryClient.invalidateQueries({
          queryKey: ["project-skills", projectId],
        });
      }
    },
    [queryClient],
  );

  return useEventSource(
    enabled ? `${API_BASE}/api/events/stream` : null,
    onEvent,
  );
}
