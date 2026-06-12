import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import {
  ConversationSessionList,
  ConversationThread,
} from "@/components/ConversationThread";
import { ConversationArtifactsPanel } from "@/components/ConversationArtifactsPanel";
import { ConversationComposer } from "@/components/ConversationComposer";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
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
import { useNavigate, useRouterState, useSearch } from "@tanstack/react-router";

export function ConversationsPage() {
  const t = useT();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const search = useSearch({ from: "/_shell/conversations" }) as ConversationSearch;
  const searchStr = useRouterState({ select: (s) => s.location.searchStr });

  const [projectId, setProjectId] = useState(search.project ?? "");
  const [showStartForm, setShowStartForm] = useState(Boolean(search.agent));
  const [artifactsDrawerOpen, setArtifactsDrawerOpen] = useState(false);
  const [sessionsDrawerOpen, setSessionsDrawerOpen] = useState(false);
  const [listCollapsed, setListCollapsed] = useState(false);
  /** Instant list highlight while router search catches up. */
  const [pendingSessionId, setPendingSessionId] = useState<string | null>(null);
  const globalSseLive = useSseStatus() === "live";
  const { counts: pendingCounts, pendingTotal, isLoading: pendingCountsLoading } =
    usePendingApprovalCounts();

  /** Chip + list query follow the URL bar immediately (not lagging useSearch). */
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
      // Router navigate merges stray query keys — replace the browser URL first.
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
      queueMicrotask(() => {
        navigateSearch({
          ...effectiveSearch,
          session: sessionId || undefined,
        });
      });
    },
    [effectiveSearch, navigateSearch],
  );

  const renderStartComposer = (compact?: boolean) => (
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

  /** URL-derived selection (shareable / refresh). */
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
    const chips = [
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
  }, [facets.data?.facets.budget_exceeded_7d, facets.data?.facets.kind, facets.data?.facets.pending_approval_total, pendingTotal, t]);

  const applyChip = (chipId: string) => {
    setPendingSessionId(null);
    navigateSearch({
      project: projectId || search.project || undefined,
      agent: search.agent,
      filter: chipId === "all" ? undefined : chipId,
    });
  };

  const listBusy = sessions.isFetching;

  return (
    <div className="flex flex-col min-h-[calc(100vh-9rem)]">
      <div className="flex flex-wrap items-center gap-2 mb-3 shrink-0">
        <select
          className="dw-input min-w-[12rem]"
          value={projectId}
          onChange={(e) => {
            const nextProject = e.target.value;
            setProjectId(nextProject);
            navigateSearch({
              ...effectiveSearch,
              project: nextProject || undefined,
              session: undefined,
            });
          }}
        >
          <option value="">{t("conversations.allProjects")}</option>
          {(projects.data?.projects ?? []).map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        {quickChips.map((f) => (
          <button
            key={f.id}
            type="button"
            className={`dw-chip${active === f.id ? " active" : ""}`}
            onClick={() => applyChip(f.id)}
          >
            {f.label}
            {f.badge != null && Number(f.badge) > 0 && (
              <span className="ml-1 rounded-full bg-warn/20 text-warn px-1.5 text-[10px]">
                {f.badge}
              </span>
            )}
          </button>
        ))}
        {projectId && (
          <button
            type="button"
            className="dw-btn-primary ml-auto"
            onClick={() => setShowStartForm((v) => !v)}
          >
            {t("conversations.newSession")}
          </button>
        )}
      </div>

      {sessions.isError && (
        <div className="dw-alert-error mb-4">
          <p className="m-0 font-medium">{t("common.error")}</p>
          <p className="m-0 mt-1 text-sm">{(sessions.error as Error).message}</p>
        </div>
      )}

      {sessions.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}

      {!sessions.isLoading &&
        active === "needs_approval" &&
        pendingCountsLoading &&
        rows.length === 0 && (
          <p className="text-sm text-secondary">{t("common.loading")}</p>
        )}

      {!sessions.isLoading &&
        !(active === "needs_approval" && pendingCountsLoading) &&
        !projectId &&
        rows.length === 0 &&
        active === "all" && (
        <EmptyState
          title={t("conversations.selectProjectFirst")}
          description={t("conversations.selectProjectFirstDesc")}
          icon="folder_open"
        />
      )}

      {!sessions.isLoading &&
        !(active === "needs_approval" && pendingCountsLoading) &&
        projectId &&
        rows.length === 0 &&
        active === "all" && (
        <div className="p-6 border border-outline-variant rounded-lg bg-surface-container-lowest">
          {!showStartForm && (
            <EmptyState
              title={t("conversations.emptyTitle")}
              description={t("conversations.emptyDesc")}
              icon="forum"
            />
          )}
          {showStartForm ? (
            renderStartComposer()
          ) : (
            <div className="text-center mt-4">
              <button
                type="button"
                className="dw-btn-primary"
                onClick={() => setShowStartForm(true)}
              >
                {t("conversations.newSession")}
              </button>
            </div>
          )}
        </div>
      )}

      {!sessions.isLoading &&
        !(active === "needs_approval" && pendingCountsLoading) &&
        rows.length === 0 &&
        active !== "all" && (
        <EmptyState
          title={
            active === "needs_approval"
              ? t("conversations.emptyNeedsApproval")
              : t("conversations.emptyFilter")
          }
          description={
            active === "needs_approval" ? t("conversations.emptyNeedsApprovalDesc") : undefined
          }
          icon="forum"
        />
      )}

      {projectId && showStartForm && rows.length > 0 && (
        <div className="mb-4">{renderStartComposer(true)}</div>
      )}

      {rows.length > 0 && (
        <div className="flex flex-col flex-1 min-h-0 border border-outline-variant rounded-lg overflow-hidden bg-surface-container-lowest min-h-[calc(100vh-12rem)]">
          <div className="lg:hidden flex items-center justify-between gap-2 px-3 py-2 border-b border-outline-variant bg-surface-container-low shrink-0">
            <button
              type="button"
              className="dw-btn-secondary text-xs"
              onClick={() => setSessionsDrawerOpen(true)}
            >
              <Icon name="forum" size={16} />
              {t("conversations.sessionList")}
            </button>
            <button
              type="button"
              className="dw-btn-secondary text-xs"
              onClick={() => setArtifactsDrawerOpen(true)}
            >
              <Icon name="inventory_2" size={16} />
              {t("conversations.artifactsPanel")}
            </button>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-12 flex-1 min-h-0">
            {!listCollapsed && (
              <div className="hidden lg:flex lg:col-span-3 border-r border-outline-variant flex-col min-h-0">
                <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0 flex items-center justify-between gap-2">
                  <span>
                    {t("conversations.sessionList")} ({rows.length})
                  </span>
                  <button
                    type="button"
                    className="dw-btn-ghost p-1"
                    title={t("conversations.listCollapse")}
                    onClick={() => setListCollapsed(true)}
                  >
                    <Icon name="chevron_left" size={18} />
                  </button>
                </div>
                <div
                  className={`flex-1 min-h-0 overflow-y-auto transition-opacity ${listBusy ? "opacity-60 pointer-events-none" : ""}`}
                >
                  <ConversationSessionList
                    sessions={rows}
                    selectedId={displaySessionId}
                    onSelect={selectSession}
                    pendingCounts={pendingCounts}
                    onPrefetch={(id, isRunning) =>
                      prefetchSessionConversation(queryClient, id, isRunning)
                    }
                  />
                </div>
              </div>
            )}
            <div
              className={`flex flex-col min-h-0 border-outline-variant ${
                listCollapsed ? "lg:col-span-9 lg:border-r" : "lg:col-span-6 lg:border-r"
              }`}
            >
              {listCollapsed && (
                <div className="hidden lg:flex px-3 py-2 border-b border-outline-variant bg-surface-container-low shrink-0">
                  <button
                    type="button"
                    className="dw-btn-secondary text-xs"
                    onClick={() => setListCollapsed(false)}
                  >
                    <Icon name="chevron_right" size={16} />
                    {t("conversations.listExpand")}
                  </button>
                </div>
              )}
              <div className="flex-1 min-h-0 flex flex-col">
                <ConversationThread
                  session={selected}
                  onFollowUpStarted={selectSession}
                  showHeader={false}
                  sseLive={sseLive}
                />
              </div>
            </div>
            <div className="hidden lg:flex lg:col-span-3 flex-col min-h-0">
              <ConversationArtifactsPanel
                sessionId={displaySessionId}
                live={sseLive}
                isRunning={selected?.status === "running"}
              />
            </div>
          </div>
        </div>
      )}

      {sessionsDrawerOpen && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-40 bg-black/30 lg:hidden"
            aria-label={t("common.back")}
            onClick={() => setSessionsDrawerOpen(false)}
          />
          <div className="fixed inset-y-0 left-0 z-50 w-[min(100%,20rem)] lg:hidden shadow-xl">
            <div className="h-full border-r border-outline-variant bg-surface-container-lowest flex flex-col">
              <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0 flex items-center justify-between">
                <span>{t("conversations.sessionList")}</span>
                <button
                  type="button"
                  className="dw-btn-ghost p-1"
                  onClick={() => setSessionsDrawerOpen(false)}
                >
                  <Icon name="close" size={18} />
                </button>
              </div>
              <div className="flex-1 min-h-0 overflow-y-auto">
                <ConversationSessionList
                  sessions={rows}
                  selectedId={displaySessionId}
                  onSelect={(id) => {
                    selectSession(id);
                    setSessionsDrawerOpen(false);
                  }}
                  pendingCounts={pendingCounts}
                  onPrefetch={(id, isRunning) =>
                    prefetchSessionConversation(queryClient, id, isRunning)
                  }
                />
              </div>
            </div>
          </div>
        </>
      )}

      {artifactsDrawerOpen && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-40 bg-black/30 lg:hidden"
            aria-label={t("common.back")}
            onClick={() => setArtifactsDrawerOpen(false)}
          />
          <div className="fixed inset-y-0 right-0 z-50 w-[min(100%,20rem)] lg:hidden shadow-xl">
            <ConversationArtifactsPanel
              sessionId={displaySessionId}
              live={sseLive}
              isRunning={selected?.status === "running"}
              className="h-full border-l border-outline-variant"
            />
            <button
              type="button"
              className="absolute top-2 right-2 dw-btn-ghost p-1.5"
              onClick={() => setArtifactsDrawerOpen(false)}
            >
              <Icon name="close" size={18} />
            </button>
          </div>
        </>
      )}
    </div>
  );
}
