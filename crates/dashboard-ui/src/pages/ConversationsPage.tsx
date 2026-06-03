import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { api, type SessionListOpts } from "@/api/client";
import {
  ConversationSessionList,
  ConversationThread,
} from "@/components/ConversationThread";
import { ConversationArtifactsPanel } from "@/components/ConversationArtifactsPanel";
import { ConversationComposer } from "@/components/ConversationComposer";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/ui/PageHeader";
import { SessionStatusBadges } from "@/components/ui/StatusBadge";
import { usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { useSessionEventStream } from "@/hooks/useSessionEventStream";
import { useT } from "@/i18n/context";

type ConversationSearch = {
  status?: string;
  trusted?: string;
  kind?: string;
  needs_approval?: boolean;
  budget_exceeded?: boolean;
  project?: string;
  session?: string;
  agent?: string;
  filter?: "all" | "running" | "blocked" | "workflow" | "cron" | "needs_approval" | "budget";
};

function legacyFilterToSearch(filter: ConversationSearch["filter"]): Partial<ConversationSearch> {
  switch (filter) {
    case "running":
      return { status: "running" };
    case "blocked":
      return { trusted: "blocked" };
    case "workflow":
      return { kind: "workflow" };
    case "cron":
      return { kind: "cron" };
    case "needs_approval":
      return { status: "running", needs_approval: true };
    case "budget":
      return { budget_exceeded: true };
    default:
      return {};
  }
}

function searchToSessionOpts(search: ConversationSearch): SessionListOpts {
  return {
    limit: 100,
    status: search.status,
    trustedStatus: search.trusted,
    kind: search.kind,
    projectId: search.project,
    budgetExceeded: search.budget_exceeded,
  };
}

function activeChip(search: ConversationSearch): string {
  if (search.needs_approval) return "needs_approval";
  if (search.status === "running" && !search.kind && !search.trusted) return "running";
  if (search.trusted === "blocked") return "blocked";
  if (search.kind === "workflow") return "workflow";
  if (search.kind === "cron") return "cron";
  if (search.budget_exceeded) return "budget";
  if (search.kind) return `kind:${search.kind}`;
  if (!search.status && !search.trusted && !search.kind && !search.needs_approval) return "all";
  return "custom";
}

export function ConversationsPage() {
  const t = useT();
  const navigate = useNavigate();
  const rawSearch = useSearch({ from: "/_shell/conversations" }) as ConversationSearch;
  const search = useMemo(() => {
    if (rawSearch.filter && !rawSearch.status && !rawSearch.trusted && !rawSearch.kind) {
      return { ...rawSearch, ...legacyFilterToSearch(rawSearch.filter), filter: undefined };
    }
    return rawSearch;
  }, [rawSearch]);

  const [projectId, setProjectId] = useState(search.project ?? "");
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(search.session ?? null);
  const [showStartForm, setShowStartForm] = useState(Boolean(search.agent));
  const [artifactsDrawerOpen, setArtifactsDrawerOpen] = useState(false);
  const { counts: pendingCounts, pendingTotal } = usePendingApprovalCounts();
  const active = activeChip(search);

  const updateSearch = useCallback(
    (next: Partial<ConversationSearch> & { sessionId?: string | null }) => {
      const merged: ConversationSearch = {
        ...search,
        ...next,
        session: next.sessionId === undefined ? search.session : next.sessionId || undefined,
      };
      if ("sessionId" in next) delete (merged as { sessionId?: string | null }).sessionId;
      void navigate({
        to: "/conversations",
        search: {
          status: merged.status,
          trusted: merged.trusted,
          kind: merged.kind,
          needs_approval: merged.needs_approval || undefined,
          budget_exceeded: merged.budget_exceeded || undefined,
          project: merged.project || undefined,
          session: merged.session || undefined,
          agent: merged.agent || undefined,
        },
        replace: true,
      });
    },
    [navigate, search],
  );

  const selectSession = useCallback(
    (sessionId: string | null) => {
      setSelectedSessionId(sessionId);
      updateSearch({ sessionId });
    },
    [updateSearch],
  );

  useEffect(() => {
    if (search.project) {
      setProjectId(search.project);
    }
  }, [search.project]);

  useEffect(() => {
    if (search.session) setSelectedSessionId(search.session);
  }, [search.session]);

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
    queryKey: ["all-sessions", search.status, search.trusted, search.kind, search.budget_exceeded, projectId],
    queryFn: () => api.allSessions(searchToSessionOpts({ ...search, project: projectId || undefined })),
    refetchInterval: 3_000,
  });

  const rows = useMemo(() => {
    const base = sessions.data?.sessions ?? [];
    if (search.needs_approval) {
      return base.filter((s) => s.status === "running" && (pendingCounts.get(s.id) ?? 0) > 0);
    }
    return base;
  }, [pendingCounts, search.needs_approval, sessions.data?.sessions]);

  useEffect(() => {
    if (rows.length === 0 || sessions.isLoading) return;
    if (!selectedSessionId) {
      selectSession(rows[0].id);
      return;
    }
    if (!search.session && !rows.some((s) => s.id === selectedSessionId)) {
      selectSession(rows[0].id);
    }
  }, [rows, search.session, selectSession, selectedSessionId, sessions.isLoading]);

  const selected = useMemo(
    () => rows.find((s) => s.id === selectedSessionId) ?? null,
    [rows, selectedSessionId],
  );

  const sseLive = useSessionEventStream(selectedSessionId ?? undefined);

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
    if (chipId === "all") {
      updateSearch({
        status: undefined,
        trusted: undefined,
        kind: undefined,
        needs_approval: undefined,
        budget_exceeded: undefined,
      });
      return;
    }
    if (chipId === "running") {
      updateSearch({
        status: "running",
        trusted: undefined,
        kind: undefined,
        needs_approval: undefined,
        budget_exceeded: undefined,
      });
      return;
    }
    if (chipId === "blocked") {
      updateSearch({
        trusted: "blocked",
        status: undefined,
        kind: undefined,
        needs_approval: undefined,
        budget_exceeded: undefined,
      });
      return;
    }
    if (chipId === "needs_approval") {
      updateSearch({
        status: "running",
        needs_approval: true,
        trusted: undefined,
        kind: undefined,
        budget_exceeded: undefined,
      });
      return;
    }
    if (chipId === "budget") {
      updateSearch({
        budget_exceeded: true,
        status: undefined,
        trusted: undefined,
        kind: undefined,
        needs_approval: undefined,
      });
      return;
    }
    if (chipId.startsWith("kind:")) {
      updateSearch({
        kind: chipId.slice("kind:".length),
        status: undefined,
        trusted: undefined,
        needs_approval: undefined,
        budget_exceeded: undefined,
      });
    }
  };

  return (
    <>
      <PageHeader
        title={t("conversations.title")}
        subtitle={t("conversations.subtitleChat")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("conversations.title") },
        ]}
        meta={
          selected ? (
            <>
              <SessionStatusBadges
                status={selected.status}
                trustedStatus={selected.trusted_status}
                pendingApprovalCount={pendingCounts.get(selected.id)}
              />
            </>
          ) : undefined
        }
      />
      <div className="flex flex-wrap items-center gap-2 mb-4">
        <select
          className="dw-input min-w-[12rem]"
          value={projectId}
          onChange={(e) => {
            const nextProject = e.target.value;
            setProjectId(nextProject);
            setSelectedSessionId(null);
            updateSearch({ project: nextProject || undefined, sessionId: null });
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

      {!sessions.isLoading && !projectId && rows.length === 0 && active === "all" && (
        <EmptyState
          title={t("conversations.selectProjectFirst")}
          description={t("conversations.selectProjectFirstDesc")}
          icon="folder_open"
        />
      )}

      {!sessions.isLoading && projectId && rows.length === 0 && active === "all" && (
        <div className="p-6 border border-outline-variant rounded-lg bg-surface-container-lowest">
          {!showStartForm && (
            <EmptyState
              title={t("conversations.emptyTitle")}
              description={t("conversations.emptyDesc")}
              icon="forum"
            />
          )}
          {showStartForm ? (
            <ConversationComposer
              mode="start"
              projectId={projectId}
              initialAgent={search.agent}
              onSuccess={({ session }) => {
                setShowStartForm(false);
                selectSession(session.id);
              }}
              onCancel={() => setShowStartForm(false)}
            />
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

      {!sessions.isLoading && rows.length === 0 && active !== "all" && (
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
        <div className="mb-4">
          <ConversationComposer
            mode="start"
            projectId={projectId}
            initialAgent={search.agent}
            compact
            onSuccess={({ session }) => {
              setShowStartForm(false);
              selectSession(session.id);
            }}
            onCancel={() => setShowStartForm(false)}
          />
        </div>
      )}

      {rows.length > 0 && (
        <div className="flex flex-col flex-1 min-h-0 border border-outline-variant rounded-lg overflow-hidden bg-surface-container-lowest">
          <div className="lg:hidden flex items-center justify-end gap-2 px-3 py-2 border-b border-outline-variant bg-surface-container-low shrink-0">
            <button
              type="button"
              className="dw-btn-secondary text-xs"
              onClick={() => setArtifactsDrawerOpen(true)}
            >
              <Icon name="inventory_2" size={16} />
              {t("conversations.artifactsPanel")}
            </button>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-12 flex-1 min-h-0 min-h-[min(720px,calc(100vh-16rem))]">
            <div className="lg:col-span-3 border-b lg:border-b-0 lg:border-r border-outline-variant flex flex-col min-h-0 max-h-[40vh] lg:max-h-none">
              <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0">
                {t("conversations.sessionList")} ({rows.length})
              </div>
              <div className="flex-1 min-h-0 overflow-y-auto">
                <ConversationSessionList
                  sessions={rows}
                  selectedId={selectedSessionId}
                  onSelect={selectSession}
                  pendingCounts={pendingCounts}
                />
              </div>
            </div>
            <div className="lg:col-span-6 flex flex-col min-h-0 border-b lg:border-b-0 lg:border-r border-outline-variant">
              <ConversationThread
                session={selected}
                onFollowUpStarted={selectSession}
              />
            </div>
            <div className="hidden lg:flex lg:col-span-3 flex-col min-h-0">
              <ConversationArtifactsPanel
                sessionId={selectedSessionId}
                live={sseLive}
              />
            </div>
          </div>
        </div>
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
              sessionId={selectedSessionId}
              live={sseLive}
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
    </>
  );
}
