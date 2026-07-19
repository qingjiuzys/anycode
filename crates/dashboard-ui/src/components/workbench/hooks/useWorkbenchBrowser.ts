import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";

function parseApiError(err: unknown): string {
  if (!(err instanceof Error)) return String(err);
  const raw = err.message;
  try {
    const jsonStart = raw.indexOf("{");
    if (jsonStart >= 0) {
      const body = JSON.parse(raw.slice(jsonStart)) as { error?: string; message?: string };
      return body.error ?? body.message ?? raw;
    }
  } catch {
    /* keep raw */
  }
  return raw;
}

export function useWorkbenchBrowser(
  projectId: string | null | undefined,
  conversationSessionId: string | null | undefined,
  active: boolean,
) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [urlInput, setUrlInput] = useState("https://example.com");
  const [lockState, setLockState] = useState<string>("idle");
  const [frameBase64, setFrameBase64] = useState<string | null>(null);
  const sessionRef = useRef<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [createError, setCreateError] = useState<Error | null>(null);
  const [navPending, setNavPending] = useState(false);

  const status = useQuery({
    queryKey: ["workbench-browser-status"],
    queryFn: api.browserStatus,
    enabled: active,
    staleTime: 30_000,
  });

  const chromiumReady = status.data?.ready ?? status.data?.chromium_ready ?? false;
  const browserEnabled = status.data?.enabled ?? false;
  const canUseBrowser = chromiumReady;
  const shouldCreateSession = active && Boolean(projectId) && canUseBrowser;

  useEffect(() => {
    if (!shouldCreateSession) {
      setCreateError(null);
      return;
    }
    let cancelled = false;
    setCreateError(null);
    void api
      .createBrowserSession(projectId!, conversationSessionId ?? undefined)
      .then(
        (data) => {
          if (cancelled) {
            void api.deleteBrowserSession(data.session.session_id);
            return;
          }
          sessionRef.current = data.session.session_id;
          setSessionId(data.session.session_id);
        },
        (e) => setCreateError(new Error(parseApiError(e))),
      );
    return () => {
      cancelled = true;
      const sid = sessionRef.current;
      sessionRef.current = null;
      setSessionId(null);
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      if (sid) void api.deleteBrowserSession(sid);
    };
  }, [shouldCreateSession, projectId, conversationSessionId]);

  useEffect(() => {
    if (!active || !sessionId) return;
    const poll = () => {
      void api.browserState(sessionId).then((r) => {
        if (r.state.lock) setLockState(r.state.lock);
        if (r.state.url) setUrlInput(r.state.url);
      });
    };
    poll();
    const id = window.setInterval(poll, 2000);
    return () => window.clearInterval(id);
  }, [active, sessionId]);

  useEffect(() => {
    if (!active || !sessionId) return;
    const ws = new WebSocket(api.browserStreamUrl(sessionId));
    wsRef.current = ws;
    ws.onmessage = (ev) => {
      try {
        const data = JSON.parse(String(ev.data)) as {
          image_base64?: string;
          format?: string;
        };
        if (data.image_base64) {
          const mime = data.format === "jpeg" ? "image/jpeg" : "image/png";
          setFrameBase64(`${mime}:${data.image_base64}`);
        }
      } catch {
        /* ignore */
      }
    };
    return () => {
      ws.close();
      if (wsRef.current === ws) wsRef.current = null;
    };
  }, [active, sessionId]);

  const navigate = {
    isPending: navPending,
    mutate: (url: string) => {
      const sid = sessionRef.current;
      if (!sid) return;
      setNavPending(true);
      void api
        .browserLock(sid, "user")
        .then(() => api.navigateBrowser(sid, url))
        .then((result) => {
          setUrlInput(result.state.url);
          setLockState(result.state.lock ?? "user");
        })
        .finally(() => setNavPending(false));
    },
  };

  const unlockForUser = () => {
    const sid = sessionRef.current;
    if (!sid) return;
    void api.browserLock(sid, "user").then((r) => setLockState(r.lock));
  };

  return {
    urlInput,
    setUrlInput,
    navigate,
    lockState,
    unlockForUser,
    screenshot: {
      data: frameBase64
        ? {
            screenshot: {
              image_base64: frameBase64.includes(":")
                ? frameBase64.split(":").slice(1).join(":")
                : frameBase64,
              mime: frameBase64.startsWith("image/jpeg:") ? "image/jpeg" : "image/png",
            },
          }
        : null,
    },
    createSession: {
      isPending: shouldCreateSession && !sessionId && !createError,
      isError: Boolean(createError),
      error: createError,
    },
    sessionReady: Boolean(sessionId),
    status: {
      isLoading: status.isLoading,
      chromiumReady,
      browserEnabled,
      canUseBrowser,
      doctorMessage: status.data?.doctor_message,
    },
  };
}
