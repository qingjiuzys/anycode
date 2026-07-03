import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { apiUrl } from "@/api/http";

/**
 * Subscribe to per-project SSE and invalidate queries when CLI/runtime writes events.
 * Requires Workbench (desktop or anycode-dashboard-serve) running; optional `ANYCODE_DASHBOARD_URL` for cross-process notify.
 */
export function useProjectEventStream(projectId: string | undefined) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!projectId) {
      return;
    }
    const url = apiUrl(`/api/projects/${projectId}/events/stream`);
    const es = new EventSource(url);
    const refresh = () => {
      queryClient.invalidateQueries({ queryKey: ["events", projectId] });
      queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      queryClient.invalidateQueries({ queryKey: ["gates", projectId] });
      queryClient.invalidateQueries({ queryKey: ["project-stats", projectId] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
    };
    es.addEventListener("project_event", refresh);
    es.onerror = () => {
      /* EventSource retries; polling still works if server is down */
    };
    return () => es.close();
  }, [projectId, queryClient]);
}
