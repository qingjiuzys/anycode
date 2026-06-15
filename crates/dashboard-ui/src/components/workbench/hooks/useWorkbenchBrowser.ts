import { useEffect, useRef, useState } from "react";
import { api } from "@/api/client";

export function useWorkbenchBrowser(projectId: string | null | undefined, active: boolean) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [urlInput, setUrlInput] = useState("https://example.com");
  const sessionRef = useRef<string | null>(null);
  const [createError, setCreateError] = useState<Error | null>(null);
  const [navPending, setNavPending] = useState(false);
  const [shotKey, setShotKey] = useState(0);
  const [screenshot, setScreenshot] = useState<Awaited<
    ReturnType<typeof api.browserScreenshot>
  > | null>(null);

  useEffect(() => {
    if (!active || !projectId) return;
    let cancelled = false;
    setCreateError(null);
    void api.createBrowserSession(projectId).then(
      (data) => {
        if (cancelled) {
          void api.deleteBrowserSession(data.session.session_id);
          return;
        }
        sessionRef.current = data.session.session_id;
        setSessionId(data.session.session_id);
      },
      (e) => setCreateError(e instanceof Error ? e : new Error(String(e))),
    );
    return () => {
      cancelled = true;
      const sid = sessionRef.current;
      sessionRef.current = null;
      setSessionId(null);
      if (sid) void api.deleteBrowserSession(sid);
    };
  }, [active, projectId]);

  useEffect(() => {
    if (!active || !sessionId) return;
    let cancelled = false;
    const tick = async () => {
      try {
        const shot = await api.browserScreenshot(sessionId);
        if (!cancelled) setScreenshot(shot);
      } catch {
        /* ignore poll errors */
      }
    };
    void tick();
    const id = window.setInterval(tick, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [active, sessionId, shotKey]);

  const navigate = {
    isPending: navPending,
    mutate: (url: string) => {
      const sid = sessionRef.current;
      if (!sid) return;
      setNavPending(true);
      void api
        .navigateBrowser(sid, url)
        .then((result) => {
          setUrlInput(result.state.url);
          setShotKey((k) => k + 1);
        })
        .finally(() => setNavPending(false));
    },
  };

  return {
    urlInput,
    setUrlInput,
    navigate,
    screenshot: { data: screenshot },
    createSession: {
      isPending: active && Boolean(projectId) && !sessionId && !createError,
      isError: Boolean(createError),
      error: createError,
    },
    sessionReady: Boolean(sessionId),
  };
}
