import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  useSyncExternalStore,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState, useSearch } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { SessionDetail, SessionWithProject, TranscriptBlock } from "@/api/types";
import { usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { shouldClearOptimisticOnRunningHandoff, useSessionEventStream } from "@/hooks/useSessionEventStream";
import { setActiveSessionForGlobalSse } from "@/lib/activeSessionSse";
import { useSseStatus } from "@/context/SseContext";
import { useT } from "@/i18n/context";
import {
  buildConversationsHref,
  conversationSearchParams,
  conversationsCanonicalHref,
  filterToQuerySearch,
  parseConversationSearch,
  parseFilterFromSearchStr,
  searchToSessionOpts,
  type ConversationSearch,
} from "@/lib/conversationsSearch";
import { deriveSessionLiveState, type SessionLiveState } from "@/lib/sessionLiveStore";
import {
  getOptimisticResolvedApprovalIds,
  optimisticResolvedApprovalsSnapshot,
  subscribeOptimisticResolvedApprovals,
} from "@/lib/approvalOptimisticStore";
import { prefetchSessionConversation } from "@/lib/sessionQuery";

export type QuickChip = {
  id: string;
  label: string;
  badge?: number;
};

type ConversationShellContextValue = {
  projectId: string;
  setProjectId: (id: string) => void;
  workbenchDrawerOpen: boolean;
  setWorkbenchDrawerOpen: (v: boolean) => void;
  sessionsDrawerOpen: boolean;
  setSessionsDrawerOpen: (v: boolean) => void;
  sessionSidebarCollapsed: boolean;
  setSessionSidebarCollapsed: (v: boolean) => void;
  selectedTool: TranscriptBlock | null;
  setSelectedTool: (tool: TranscriptBlock | null) => void;
  active: string;
  quickChips: QuickChip[];
  applyChip: (chipId: string) => void;
  listSearch: string;
  setListSearch: (value: string) => void;
  filteredRows: SessionWithProject[];
  sidebarRows: SessionWithProject[];
  sidebarFilteredRows: SessionWithProject[];
  rows: SessionWithProject[];
  displaySessionId: string | null;
  selected: SessionWithProject | null;
  selectSession: (sessionId: string | null) => void;
  pendingCounts: Map<string, number>;
  listBusy: boolean;
  sessionsLoading: boolean;
  sessionsError: Error | null;
  pendingCountsLoading: boolean;
  sseLive: boolean;
  liveBlocks: TranscriptBlock[];
  liveEvents: import("@/lib/liveTranscript").ChatStreamEvent[];
  chatStreamLive: boolean;
  sessionLive: SessionLiveState;
  questionsRespondAllowed: boolean;
  approvalsRespondAllowed: boolean;
  sseStatus: import("@/hooks/useEventSource").SseStatus;
  isOptimisticStreaming: boolean;
  optimisticStreamingSessionId: string | null;
  onRenameSession: (sessionId: string, title: string) => void;
  onRenameProject: (projectId: string, name: string) => void;
  onRemoveProject: (projectId: string) => void;
  markSessionStreaming: (sessionId: string) => void;
  clearOptimisticStreaming: () => void;
  beginPendingSession: (session: SessionDetail, project?: { id: string; name?: string }) => void;
  projectOptions: Array<{ id: string; name: string; root_path?: string; updated_at?: string }>;
  projectsError: Error | null;
  navigateSearch: (next: ConversationSearch) => void;
  effectiveSearch: ConversationSearch;
  search: ConversationSearch;
  prefetchSession: (id: string, isRunning: boolean) => void;
  startSessionForProject: (projectId: string) => void;
  goHome: (projectId?: string) => void;
};

const ConversationShellContext = createContext<ConversationShellContextValue | null>(null);

export function ConversationShellProvider({ children }: { children: React.ReactNode }) {
  const value = useConversationShellState();
  return (
    <ConversationShellContext.Provider value={value}>{children}</ConversationShellContext.Provider>
  );
}

export function useConversationShell(): ConversationShellContextValue {
  const ctx = useContext(ConversationShellContext);
  if (!ctx) {
    throw new Error("useConversationShell must be used within ConversationShellProvider");
  }
  return ctx;
}

function useConversationShellState(): ConversationShellContextValue {
  const t = useT();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const searchStr = useRouterState({ select: (s) => s.location.searchStr });
  const conversationsSearch = useSearch({
    from: "/_shell/conversations",
    shouldThrow: false,
  }) as ConversationSearch | undefined;
  const homeSearch = useSearch({
    from: "/_shell/",
    shouldThrow: false,
  }) as { project?: string } | undefined;
  const search = useMemo((): ConversationSearch => {
    if (pathname === "/conversations") {
      return conversationsSearch ?? parseConversationSearch(searchStr);
    }
    if (pathname === "/") {
      return parseConversationSearch(searchStr);
    }
    return {};
  }, [conversationsSearch, homeSearch, pathname, searchStr]);

  const [projectId, setProjectId] = useState(search.project ?? homeSearch?.project ?? "");
  const [workbenchDrawerOpen, setWorkbenchDrawerOpen] = useState(false);
  const [sessionsDrawerOpen, setSessionsDrawerOpen] = useState(false);
  const [sessionSidebarCollapsed, setSessionSidebarCollapsedState] = useState(() => {
    try {
      return localStorage.getItem("anycode-session-sidebar-collapsed") === "1";
    } catch {
      return false;
    }
  });
  const setSessionSidebarCollapsed = useCallback((v: boolean) => {
    setSessionSidebarCollapsedState(v);
    try {
      localStorage.setItem("anycode-session-sidebar-collapsed", v ? "1" : "0");
    } catch {
      /* ignore */
    }
  }, []);
  const [selectedTool, setSelectedTool] = useState<TranscriptBlock | null>(null);
  const [pendingSessionId, setPendingSessionId] = useState<string | null>(null);
  const [pendingSessionMeta, setPendingSessionMeta] = useState<SessionWithProject | null>(null);
  const [listSearch, setListSearch] = useState("");
  const [optimisticStreamingSessionId, setOptimisticStreamingSessionId] = useState<string | null>(
    null,
  );
  const globalSseLive = useSseStatus() === "live";
  const { counts: pendingCounts, pendingTotal, isLoading: pendingCountsLoading } =
    usePendingApprovalCounts();

  const active = useMemo(() => parseFilterFromSearchStr(searchStr), [searchStr]);

  const effectiveSearch = useMemo((): ConversationSearch => {
    const fromFilter = filterToQuerySearch(active);
    return {
      ...fromFilter,
      project: search.project,
      session: search.session,
      agent: search.agent,
    };
  }, [active, search.agent, search.project, search.session]);

  const navigateSearch = useCallback(
    (next: ConversationSearch) => {
      const canon = conversationSearchParams(next);
      const href = buildConversationsHref(canon);
      window.history.replaceState(window.history.state, "", href);
      void navigate({
        to: "/conversations",
        search: () => canon,
      });
    },
    [navigate],
  );

  useEffect(() => {
    const canonicalHref = conversationsCanonicalHref(searchStr);
    if (!canonicalHref) return;
    const current = `${window.location.pathname}${window.location.search}`;
    if (canonicalHref === current) return;
    window.history.replaceState(window.history.state, "", canonicalHref);
    const canon = conversationSearchParams(
      parseConversationSearch(canonicalHref.split("?")[1] ?? ""),
    );
    void navigate({
      to: "/conversations",
      search: () => canon,
    });
  }, [navigate, searchStr]);

  const goHome = useCallback(
    (nextProjectId?: string) => {
      setPendingSessionId(null);
      setSelectedTool(null);
      if (nextProjectId) {
        setProjectId(nextProjectId);
      }
      void navigate({
        to: "/",
        search: nextProjectId ? { project: nextProjectId } : {},
      });
    },
    [navigate],
  );

  const sidebarSessions = useQuery({
    queryKey: ["all-sessions", "sidebar"],
    queryFn: () => api.allSessions({ limit: 200 }),
    staleTime: 8_000,
    refetchInterval: globalSseLive ? false : 30_000,
    refetchIntervalInBackground: false,
  });

  const sidebarRows = sidebarSessions.data?.sessions ?? [];

  const selectSession = useCallback(
    (sessionId: string | null) => {
      setSelectedTool(null);
      if (!sessionId) {
        setPendingSessionId(null);
        queueMicrotask(() => {
          navigateSearch({
            ...effectiveSearch,
            session: undefined,
          });
        });
        return;
      }
      setPendingSessionId(sessionId);
      queueMicrotask(() => {
        const hit = sidebarRows.find((s) => s.id === sessionId);
        navigateSearch({
          ...effectiveSearch,
          project: hit?.project_id ?? effectiveSearch.project,
          session: sessionId,
        });
      });
    },
    [effectiveSearch, navigateSearch, sidebarRows],
  );

  useEffect(() => {
    const fromUrl = search.project ?? homeSearch?.project;
    if (fromUrl) {
      setProjectId(fromUrl);
    }
  }, [homeSearch?.project, search.project]);

  const projects = useQuery({
    queryKey: ["projects", "picker"],
    queryFn: () => api.projects({ limit: 200, sort: "updated_at_desc" }),
  });
  const facets = useQuery({
    queryKey: ["session-facets"],
    queryFn: api.sessionFacets,
    staleTime: 30_000,
  });

  const sessions = useQuery({
    queryKey: ["all-sessions", active, search.project],
    queryFn: () => api.allSessions(searchToSessionOpts(effectiveSearch, search.project)),
    staleTime: 8_000,
    refetchInterval: globalSseLive
      ? false
      : active === "running" || active === "needs_approval"
        ? 20_000
        : 30_000,
    refetchIntervalInBackground: false,
  });

  const rows = useMemo(() => {
    const base = sessions.data?.sessions ?? [];
    if (active === "needs_approval") {
      return base.filter((s) => s.status === "running" && (pendingCounts.get(s.id) ?? 0) > 0);
    }
    return base;
  }, [active, pendingCounts, sessions.data?.sessions]);

  const filteredRows = useMemo(() => {
    const q = listSearch.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((s) => {
      const haystack = [s.title, s.id, s.project_name].filter(Boolean).join(" ").toLowerCase();
      return haystack.includes(q);
    });
  }, [listSearch, rows]);

  const sidebarFilteredRows = useMemo(() => {
    const q = listSearch.trim().toLowerCase();
    if (!q) return sidebarRows;
    return sidebarRows.filter((s) => {
      const haystack = [s.title, s.id, s.project_name].filter(Boolean).join(" ").toLowerCase();
      return haystack.includes(q);
    });
  }, [listSearch, sidebarRows]);

  const urlSessionId = useMemo(() => {
    if (pathname === "/") return null;
    if (search.session) return search.session;
    const pool = sidebarRows.length > 0 ? sidebarRows : rows;
    if (pool.length === 0) return null;
    return pool[0]!.id;
  }, [pathname, rows, sidebarRows, search.session]);

  useEffect(() => {
    if (pendingSessionId && search.session === pendingSessionId) {
      setPendingSessionId(null);
    }
  }, [pendingSessionId, search.session]);

  const displaySessionId = pendingSessionId ?? urlSessionId;

  const baseSelected = useMemo(() => {
    const pool = sidebarRows.length > 0 ? sidebarRows : rows;
    const hit = pool.find((s) => s.id === displaySessionId);
    if (hit) return hit;
    if (pendingSessionMeta?.id === displaySessionId) return pendingSessionMeta;
    return null;
  }, [rows, sidebarRows, displaySessionId, pendingSessionMeta]);

  const sessionDetailQuery = useQuery({
    queryKey: ["session", displaySessionId],
    queryFn: () => api.session(displaySessionId!),
    enabled: Boolean(displaySessionId),
    staleTime: 2_000,
    refetchInterval: (query) =>
      query.state.data?.session.status === "running" ? 2_000 : false,
  });

  const selected = useMemo(() => {
    const base = baseSelected;
    if (!base) return null;
    const fresh = sessionDetailQuery.data?.session;
    if (fresh?.id === base.id) {
      return { ...base, ...fresh };
    }
    return base;
  }, [baseSelected, sessionDetailQuery.data?.session]);

  useEffect(() => {
    if (!displaySessionId) return;
    const pool = sidebarRows.length > 0 ? sidebarRows : rows;
    if (pool.length === 0) return;
    const idx = pool.findIndex((s) => s.id === displaySessionId);
    if (idx < 0) return;
    const neighbors = [pool[idx - 1], pool[idx + 1]].filter(Boolean) as typeof pool;
    const runIdle = () => {
      for (const s of neighbors) {
        prefetchSessionConversation(queryClient, s.id, s.status === "running");
      }
    };
    if (typeof requestIdleCallback !== "undefined") {
      const id = requestIdleCallback(runIdle);
      return () => cancelIdleCallback(id);
    }
    const timer = setTimeout(runIdle, 200);
    return () => clearTimeout(timer);
  }, [displaySessionId, queryClient, rows, sidebarRows]);

  const runningSessionId = displaySessionId ?? undefined;

  const clearOptimisticStreaming = useCallback(() => {
    setOptimisticStreamingSessionId(null);
  }, []);

  const markSessionStreaming = useCallback((sessionId: string) => {
    setOptimisticStreamingSessionId(sessionId);
  }, []);

  const sessionStream = useSessionEventStream(runningSessionId, "conversation", {
    onTurnDone: clearOptimisticStreaming,
    sessionStatus: selected?.status,
    optimisticStreaming:
      optimisticStreamingSessionId !== null &&
      optimisticStreamingSessionId === displaySessionId,
  });

  useEffect(() => {
    setActiveSessionForGlobalSse(runningSessionId ?? null);
    return () => setActiveSessionForGlobalSse(null);
  }, [runningSessionId]);

  useEffect(() => {
    if (!displaySessionId) {
      clearOptimisticStreaming();
      return;
    }
    // Hand off to server status once the session list catches up to `running`.
    if (
      shouldClearOptimisticOnRunningHandoff(
        selected?.status,
        optimisticStreamingSessionId,
        displaySessionId,
      )
    ) {
      clearOptimisticStreaming();
    }
  }, [
    clearOptimisticStreaming,
    displaySessionId,
    optimisticStreamingSessionId,
    selected?.status,
  ]);

  const sseLive = sessionStream.connected;
  const liveBlocks = sessionStream.liveBlocks;
  const liveEvents = sessionStream.liveEvents;
  const chatStreamLive = sessionStream.live;
  const sseStatus = sessionStream.status;

  const rehydrateQuestions = useQuery({
    queryKey: ["pending-questions-rehydrate", displaySessionId],
    queryFn: () =>
      api.pendingQuestions({
        limit: 5,
        sessionId: displaySessionId ?? undefined,
      }),
    enabled: Boolean(displaySessionId),
    staleTime: 2_000,
    refetchInterval: selected?.status === "running" ? 2_000 : false,
    refetchOnWindowFocus: true,
  });

  const rehydrateApprovals = useQuery({
    queryKey: ["pending-approvals-rehydrate", displaySessionId],
    queryFn: () =>
      api.pendingApprovals({
        limit: 10,
        sessionId: displaySessionId ?? undefined,
      }),
    enabled: Boolean(displaySessionId),
    staleTime: Infinity,
    refetchInterval: false,
    refetchOnWindowFocus: false,
  });

  const optimisticResolvedSnapshot = useSyncExternalStore(
    subscribeOptimisticResolvedApprovals,
    () => optimisticResolvedApprovalsSnapshot(displaySessionId ?? undefined),
    () => "",
  );

  const sessionLive = useMemo(
    () =>
      deriveSessionLiveState(
        liveBlocks,
        liveEvents,
        rehydrateQuestions.data?.pending ?? [],
        rehydrateApprovals.data?.pending ?? [],
        displaySessionId ?? undefined,
        selected?.status === "running",
        getOptimisticResolvedApprovalIds(displaySessionId ?? undefined),
      ),
    [
      displaySessionId,
      liveBlocks,
      liveEvents,
      optimisticResolvedSnapshot,
      rehydrateApprovals.data?.pending,
      rehydrateQuestions.data?.pending,
      selected?.status,
    ],
  );

  // Re-fetch pending AskUserQuestion forms when SSE missed registration.
  useEffect(() => {
    if (!displaySessionId) return;
    if (sessionLive.pendingQuestions.length > 0) return;
    const askInFlight = liveEvents.some(
      (evt) =>
        evt.tool_name === "AskUserQuestion" &&
        (evt.kind === "tool_start" || evt.kind === "tool_progress"),
    );
    if (!askInFlight) return;
    void queryClient.invalidateQueries({
      queryKey: ["pending-questions-rehydrate", displaySessionId],
    });
  }, [
    displaySessionId,
    liveEvents,
    queryClient,
    sessionLive.pendingQuestions.length,
  ]);

  // Sidebar summary can show pending while live/rehydrate is still empty (SSE miss).
  useEffect(() => {
    if (!displaySessionId) return;
    const diskPending = pendingCounts.get(displaySessionId) ?? 0;
    if (diskPending <= 0) return;
    if (sessionLive.pendingApprovals.length > 0) return;
    if (getOptimisticResolvedApprovalIds(displaySessionId).size > 0) return;
    void queryClient.invalidateQueries({
      queryKey: ["pending-approvals-rehydrate", displaySessionId],
    });
    void queryClient.invalidateQueries({
      queryKey: ["security-approvals-pending", displaySessionId],
    });
  }, [
    displaySessionId,
    pendingCounts,
    queryClient,
    sessionLive.pendingApprovals.length,
    optimisticResolvedSnapshot,
  ]);

  const questionsRespondAllowed = rehydrateQuestions.data?.respond_allowed ?? true;
  const approvalsRespondAllowed = rehydrateApprovals.data?.respond_allowed ?? true;
  const isOptimisticStreaming =
    optimisticStreamingSessionId !== null &&
    optimisticStreamingSessionId === displaySessionId;

  const quickChips = useMemo(() => {
    const chips: QuickChip[] = [
      { id: "all", label: t("conversations.filters.all") },
      { id: "running", label: t("conversations.filters.running") },
      {
        id: "needs_approval",
        label: t("conversations.filters.needsApproval"),
        badge: facets.data?.facets.pending_approval_total ?? pendingTotal,
      },
      { id: "blocked", label: t("conversations.filters.blocked") },
      {
        id: "budget",
        label: t("conversations.filters.budgetExceeded"),
        badge: facets.data?.facets.budget_exceeded_7d ?? 0,
      },
    ];
    const known = new Set(["repl", "run", "goal"]);
    for (const item of facets.data?.facets.kind ?? []) {
      if (item.count <= 0 || known.has(item.label)) continue;
      chips.push({ id: `kind:${item.label}`, label: item.label, badge: item.count });
    }
    return chips;
  }, [
    facets.data?.facets.budget_exceeded_7d,
    facets.data?.facets.kind,
    facets.data?.facets.pending_approval_total,
    pendingTotal,
    t,
  ]);

  const applyChip = useCallback(
    (chipId: string) => {
      setPendingSessionId(null);
      navigateSearch({
        project: projectId || search.project || undefined,
        agent: search.agent,
        filter: chipId === "all" ? undefined : chipId,
      });
    },
    [navigateSearch, projectId, search.agent, search.project],
  );

  const prefetchSession = useCallback(
    (id: string, isRunning: boolean) => {
      prefetchSessionConversation(queryClient, id, isRunning);
    },
    [queryClient],
  );

  const projectOptions = useMemo(
    () =>
      (projects.data?.projects ?? [])
        .filter((p) => p.status !== "archived")
        .map((p) => ({
          id: p.id,
          name: p.name,
          root_path: p.root_path,
          updated_at: p.updated_at,
        })),
    [projects.data?.projects],
  );
  const projectsError = (projects.error as Error | null) ?? null;

  const renameProject = useCallback(
    (id: string, name: string) => {
      void (async () => {
        await api.renameProject(id, name);
        await queryClient.invalidateQueries({ queryKey: ["projects"] });
        await queryClient.invalidateQueries({ queryKey: ["project", id] });
        await queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      })();
    },
    [queryClient],
  );

  const removeProject = useCallback(
    (id: string) => {
      void (async () => {
        try {
          // Optimistic: hide from picker immediately so session rows can't
          // resurrect an archived project group in the sidebar.
          queryClient.setQueryData(
            ["projects", "picker"],
            (prev: { projects?: Array<{ id: string; status?: string }> } | undefined) => {
              if (!prev?.projects) return prev;
              return {
                ...prev,
                projects: prev.projects.map((p) =>
                  p.id === id ? { ...p, status: "archived" } : p,
                ),
              };
            },
          );
          await api.patchProjectStatus(id, "archived");
          await queryClient.invalidateQueries({ queryKey: ["projects"] });
          await queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
          if (projectId === id) {
            goHome();
          }
        } catch (err) {
          await queryClient.invalidateQueries({ queryKey: ["projects"] });
          window.alert(
            err instanceof Error ? err.message : "Failed to remove project",
          );
        }
      })();
    },
    [goHome, projectId, queryClient],
  );

  const beginPendingSession = useCallback(
    (session: SessionDetail, project?: { id: string; name?: string }) => {
      setPendingSessionId(session.id);
      const projectName =
        project?.name ??
        projectOptions.find((p) => p.id === (project?.id ?? session.project_id))?.name ??
        "";
      setPendingSessionMeta({
        id: session.id,
        project_id: project?.id ?? session.project_id,
        project_name: projectName,
        kind: session.kind,
        task_id: session.task_id ?? null,
        title: session.title,
        prompt_preview: session.prompt_preview ?? "",
        status: session.status,
        trusted_status: session.trusted_status,
        agent_type: session.agent_type,
        model: session.model,
        started_at: session.started_at,
        ended_at: session.ended_at,
      });
    },
    [projectOptions],
  );

  const startSessionForProject = useCallback(
    (nextProjectId: string) => {
      goHome(nextProjectId);
    },
    [goHome],
  );

  const renameSession = useCallback(
    (sessionId: string, title: string) => {
      void (async () => {
        await api.renameSession(sessionId, title);
        await queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
        await queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      })();
    },
    [queryClient],
  );

  return {
    projectId,
    setProjectId,
    workbenchDrawerOpen,
    setWorkbenchDrawerOpen,
    sessionsDrawerOpen,
    setSessionsDrawerOpen,
    sessionSidebarCollapsed,
    setSessionSidebarCollapsed,
    selectedTool,
    setSelectedTool,
    active,
    quickChips,
    applyChip,
    listSearch,
    setListSearch,
    filteredRows,
    sidebarRows,
    sidebarFilteredRows,
    rows,
    displaySessionId,
    selected,
    selectSession,
    pendingCounts,
    listBusy: sessions.isFetching || sidebarSessions.isFetching,
    sessionsLoading: sessions.isLoading,
    sessionsError: sessions.error as Error | null,
    pendingCountsLoading,
    sseLive,
    liveBlocks,
    liveEvents,
    chatStreamLive,
    sessionLive,
    questionsRespondAllowed,
    approvalsRespondAllowed,
    sseStatus,
    isOptimisticStreaming,
    optimisticStreamingSessionId,
    onRenameSession: renameSession,
    onRenameProject: renameProject,
    onRemoveProject: removeProject,
    markSessionStreaming,
    clearOptimisticStreaming,
    beginPendingSession,
    projectOptions,
    projectsError,
    navigateSearch,
    effectiveSearch,
    search,
    prefetchSession,
    startSessionForProject,
    goHome,
  };
}
