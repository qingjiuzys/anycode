import { useEffect, useRef, useState } from "react";

export type SseStatus = "connecting" | "live" | "reconnecting" | "offline";

/**
 * EventSource with automatic reconnect and connection status for the workbench UI.
 */
export function useEventSource(
  url: string | null,
  onProjectEvent?: (projectId?: string) => void,
): SseStatus {
  const [status, setStatus] = useState<SseStatus>(url ? "connecting" : "offline");
  const onEventRef = useRef(onProjectEvent);
  onEventRef.current = onProjectEvent;

  useEffect(() => {
    if (!url) {
      setStatus("offline");
      return;
    }

    let es: EventSource | null = null;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;
    let retries = 0;
    let cancelled = false;

    const connect = () => {
      if (cancelled) {
        return;
      }
      setStatus(retries > 0 ? "reconnecting" : "connecting");
      es = new EventSource(url);
      es.onopen = () => {
        if (!cancelled) {
          retries = 0;
          setStatus("live");
        }
      };
      es.onerror = () => {
        es?.close();
        if (cancelled) {
          return;
        }
        setStatus("reconnecting");
        const delay = Math.min(30_000, 1_000 * 2 ** Math.min(retries, 5));
        retries += 1;
        retryTimer = setTimeout(connect, delay);
      };
      const handler = (raw: Event) => {
        if (!cancelled) {
          setStatus("live");
          let projectId: string | undefined;
          try {
            const data = JSON.parse((raw as MessageEvent).data) as {
              project_id?: string;
            };
            projectId = data.project_id;
          } catch {
            /* ignore malformed SSE payload */
          }
          onEventRef.current?.(projectId);
        }
      };
      es.addEventListener("project_event", handler);
    };

    connect();

    return () => {
      cancelled = true;
      if (retryTimer) {
        clearTimeout(retryTimer);
      }
      es?.close();
      setStatus("offline");
    };
  }, [url]);

  return status;
}
