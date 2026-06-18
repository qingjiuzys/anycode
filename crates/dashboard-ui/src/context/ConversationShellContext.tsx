import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouterState, useSearch } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { SessionWithProject, TranscriptBlock } from "@/api/types";
import { ConversationComposer } from "@/components/ConversationComposer";
import { usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { useSessionEventStream } from "@/hooks/useSessionEventStream";
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
import { prefetchSessionConversation } from "@/lib/sessionQuery";

export type QuickChip = {
  id: string;
  label: string;
  badge?: number;
};

type ConversationShellContextValue = {
  projectId: string;
  setProjectId: (id: string) => void;
  showStartForm: boolean;
  setShowStartForm: Dispatch<SetStateAction<boolean>>;
  workbenchDrawerOpen: boolean;
  setWorkbenchDrawerOpen: (v: boolean) => void;
  sessionsDrawerOpen: boolean;
  setSessionsDrawerOpen: (v: boolean) => void;
  selectedTool: TranscriptBlock | null;
  setSelectedTool: (tool: TranscriptBlock | null) => void;
  active: string;
  quickChips: QuickChip[];
  applyChip: (chipId: string) => void;
  listSearch: string;
  setListSearch: (value: string) => void;
  filteredRows: SessionWithProject[];
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
  projectOptions: Array<{ id: string; name: string }>;
  navigateSearch: (next: ConversationSearch) => void;
  effectiveSearch: ConversationSearch;
  search: ConversationSearch;
  renderStartComposer: (compact?: boolean) => ReactNode;
  prefetchSession: (id: string, isRunning: boolean) => void;
};

const ConversationShellContext = createContext<ConversationShellContextValue | null>(null);

export function ConversationShellProvider({ children }: { children: ReactNode }) {
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
  const search = useSearch({ from: "/_shell/conversations" }) as ConversationSearch;
  const searchStr = useRouterState({ select: (s) => s.location.searchStr });

  const [projectId, setProjectId] = useState(search.project ?? "");
  const [showStartForm, setShowStartForm] = useState(Boolean(search.agent));
  const [workbenchDrawerOpen, setWorkbenchDrawerOpen] = useState(false);
  const [sessionsDrawerOpen, setSessionsDrawerOpen] = useState(false);
  const [selectedTool, setSelectedTool] = useState<TranscriptBlock | null>(null);
  const [pendingSessionId, setPendingSessionId] = useState<string | null>(null);
  const [listSearch, setListSearch] = useState("");
  const globalSseLive = useSseStatus() === "live";
  const { counts: pendingCounts, pendingTotal, isLoading: pendingCountsLoading } =
    usePendingApprovalCounts();

  const active = useMemo(() => parseFilterFromSearchStr(searchStr), [searchStr]);

  const effectiveSearch = useMemo((): ConversationSearch => {
    const fromFilter = filterToQuerySearch(active);
    return {
      ...fromFilter,
      project: search.project ?? (projectId || undefined),
      session: search.session,
      agent: search.agent,
    };
  }, [active, projectId, search.agent, search.project, search.session]);

  const navigateSearch = useCallback(
    (next: ConversationSearch) => {
      const canon = conversationSearchParams(next);
      const href = buildConversationsHref(canon);
      window.history.replaceState(window.history.state, "", href);
      void navigate({
        to: "/conversations",
        search: () => canon,
        replace: true,
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
      replace: true,
    });
  }, [navigate, searchStr]);

  const selectSession = useCallback(
    (sessionId: string | null) => {
      if (sessionId) {
        setPendingSessionId(sessionId);
      } else {
        setPendingSessionId(null);
      }
      setSelectedTool(null);
      queueMicrotask(() => {
        navigateSearch({
          ...effectiveSearch,
          session: sessionId || undefined,
        });
      });
    },
    [effectiveSearch, navigateSearch],
  );

  useEffect(() => {
    if (search.project) {
      setProjectId(search.project);
    }
  }, [search.project]);

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
    queryKey: ["all-sessions", active, projectId, search.project],
    queryFn: () => api.allSessions(searchToSessionOpts(effectiveSearch, projectId || undefined)),
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

  const urlSessionId = useMemo(() => {
    if (rows.length === 0) return null;
    const fromUrl = search.session;
    if (fromUrl && rows.some((s) => s.id === fromUrl)) return fromUrl;
    return rows[0]!.id;
  }, [rows, search.session]);

  useEffect(() => {
    if (pendingSessionId && search.session === pendingSessionId) {
      setPendingSessionId(null);
    }
  }, [pendingSessionId, search.session]);

  const displaySessionId = pendingSessionId ?? urlSessionId;

  const selected = useMemo(
    () => rows.find((s) => s.id === displaySessionId) ?? null,
    [rows, displaySessionId],
  );

  useEffect(() => {
    if (!displaySessionId || rows.length === 0) return;
    const idx = rows.findIndex((s) => s.id === displaySessionId);
    if (idx < 0) return;
    const neighbors = [rows[idx - 1], rows[idx + 1]].filter(Boolean) as typeof rows;
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
  }, [displaySessionId, queryClient, rows]);

  const sseLive = useSessionEventStream(
    selected?.status === "running" ? (displaySessionId ?? undefined) : undefined,
    "conversation",
  );

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

  const renderStartComposer = useCallback(
    (compact?: boolean) => (
      <ConversationComposer
        mode="start"
        projectId={projectId}
        initialAgent={search.agent}
        compact={compact}
        onSuccess={({ session }) => {
          setShowStartForm(false);
          selectSession(session.id);
        }}
        onCancel={() => setShowStartForm(false)}
      />
    ),
    [projectId, search.agent, selectSession],
  );

  const projectOptions = projects.data?.projects ?? [];

  return {
    projectId,
    setProjectId,
    showStartForm,
    setShowStartForm,
    workbenchDrawerOpen,
    setWorkbenchDrawerOpen,
    sessionsDrawerOpen,
    setSessionsDrawerOpen,
    selectedTool,
    setSelectedTool,
    active,
    quickChips,
    applyChip,
    listSearch,
    setListSearch,
    filteredRows,
    rows,
    displaySessionId,
    selected,
    selectSession,
    pendingCounts,
    listBusy: sessions.isFetching,
    sessionsLoading: sessions.isLoading,
    sessionsError: sessions.error as Error | null,
    pendingCountsLoading,
    sseLive,
    projectOptions,
    navigateSearch,
    effectiveSearch,
    search,
    renderStartComposer,
    prefetchSession,
  };
}
