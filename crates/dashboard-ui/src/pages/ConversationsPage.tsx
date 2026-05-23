import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useSearch } from "@tanstack/react-router";
import { api, type SessionListOpts } from "@/api/client";
import {
  ConversationSessionList,
  ConversationThread,
} from "@/components/ConversationThread";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { useSessionEventStream } from "@/hooks/useSessionEventStream";
import { useT } from "@/i18n/context";

type StatusFilter =
  | "all"
  | "running"
  | "blocked"
  | "workflow"
  | "cron"
  | "needs_approval";

function filterToOpts(filter: StatusFilter): SessionListOpts {
  switch (filter) {
    case "running":
    case "needs_approval":
      return { status: "running", limit: 100 };
    case "blocked":
      return { trustedStatus: "blocked", limit: 100 };
    case "workflow":
      return { kind: "workflow", limit: 100 };
    case "cron":
      return { kind: "cron", limit: 100 };
    default:
      return { limit: 100 };
  }
}

export function ConversationsPage() {
  const t = useT();
  const { filter: searchFilter } = useSearch({ from: "/_shell/conversations" });
  const [filter, setFilter] = useState<StatusFilter>(searchFilter ?? "all");
  const [projectId, setProjectId] = useState("");
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const { counts: pendingCounts, pendingTotal } = usePendingApprovalCounts();

  useEffect(() => {
    if (searchFilter) setFilter(searchFilter);
  }, [searchFilter]);

  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const opts = filterToOpts(filter);
  const sessions = useQuery({
    queryKey: ["all-sessions", filter, projectId, pendingTotal],
    queryFn: async () => {
      if (projectId) {
        const [proj, sess] = await Promise.all([
          api.project(projectId),
          api.sessions(projectId),
        ]);
        return {
          sessions: sess.sessions
            .filter((s) => matchFilter(s, filter, pendingCounts))
            .map((s) => ({
              ...s,
              project_id: projectId,
              project_name: proj.project.name,
            })),
        };
      }
      const data = await api.allSessions(opts);
      return {
        sessions: data.sessions.filter((s) => matchFilter(s, filter, pendingCounts)),
      };
    },
    refetchInterval: 3_000,
  });

  const rows = sessions.data?.sessions ?? [];

  useEffect(() => {
    if (rows.length === 0) {
      setSelectedSessionId(null);
      return;
    }
    if (!selectedSessionId || !rows.some((s) => s.id === selectedSessionId)) {
      setSelectedSessionId(rows[0].id);
    }
  }, [rows, selectedSessionId]);

  const selected = useMemo(
    () => rows.find((s) => s.id === selectedSessionId) ?? null,
    [rows, selectedSessionId],
  );

  const sseLive = useSessionEventStream(selectedSessionId ?? undefined);

  const events = useQuery({
    queryKey: ["session-events", selectedSessionId, "thread"],
    queryFn: () => api.sessionEvents(selectedSessionId!, { limit: 80 }),
    enabled: Boolean(selectedSessionId),
    refetchInterval: sseLive || selected?.status === "running" ? 2_000 : false,
  });

  const FILTERS: { id: StatusFilter; label: string; badge?: number }[] = [
    { id: "all", label: t("conversations.filters.all") },
    { id: "running", label: t("conversations.filters.running") },
    { id: "needs_approval", label: t("conversations.filters.needsApproval"), badge: pendingTotal },
    { id: "blocked", label: t("conversations.filters.blocked") },
    { id: "workflow", label: t("conversations.filters.workflow") },
    { id: "cron", label: t("conversations.filters.cron") },
  ];

  if (sessions.isError) {
    return <div className="dw-alert-error">{(sessions.error as Error).message}</div>;
  }

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
          sseLive ? (
            <>
              <StatusBadge status="running" />
              <span className="text-xs text-secondary">{t("home.live")}</span>
            </>
          ) : undefined
        }
      />
      <div className="flex flex-wrap items-center gap-2 mb-4">
        <select
          className="dw-input min-w-[12rem]"
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
        >
          <option value="">{t("conversations.allProjects")}</option>
          {(projects.data?.projects ?? []).map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        {FILTERS.map((f) => (
          <button
            key={f.id}
            type="button"
            className={`dw-chip${filter === f.id ? " active" : ""}`}
            onClick={() => setFilter(f.id)}
          >
            {f.label}
            {f.badge != null && f.badge > 0 && (
              <span className="ml-1 rounded-full bg-warn/20 text-warn px-1.5 text-[10px]">
                {f.badge}
              </span>
            )}
          </button>
        ))}
      </div>

      {sessions.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}

      {!sessions.isLoading && rows.length === 0 && (
        <EmptyState
          title={
            filter === "needs_approval"
              ? t("conversations.emptyNeedsApproval")
              : filter === "all"
                ? t("conversations.emptyTitle")
                : t("conversations.emptyFilter")
          }
          description={
            filter === "needs_approval"
              ? t("conversations.emptyNeedsApprovalDesc")
              : filter === "all"
                ? t("conversations.emptyDesc")
                : undefined
          }
          icon="forum"
        />
      )}

      {rows.length > 0 && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-0 border border-outline-variant rounded-lg overflow-hidden bg-surface-container-lowest min-h-[480px]">
          <div className="lg:col-span-4 border-b lg:border-b-0 lg:border-r border-outline-variant max-h-[520px] overflow-y-auto">
            <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low">
              {t("conversations.sessionList")} ({rows.length})
            </div>
            <ConversationSessionList
              sessions={rows}
              selectedId={selectedSessionId}
              onSelect={setSelectedSessionId}
              pendingCounts={pendingCounts}
            />
          </div>
          <div className="lg:col-span-8 bg-surface-container-lowest">
            <ConversationThread
              session={selected}
              events={events.data?.events ?? []}
              loading={events.isLoading}
            />
          </div>
        </div>
      )}
    </>
  );
}

function matchFilter(
  s: { id: string; status: string; trusted_status: string; kind: string },
  filter: StatusFilter,
  pendingCounts: Map<string, number>,
): boolean {
  switch (filter) {
    case "running":
      return s.status === "running";
    case "needs_approval":
      return s.status === "running" && (pendingCounts.get(s.id) ?? 0) > 0;
    case "blocked":
      return s.trusted_status === "blocked";
    case "workflow":
      return s.kind === "workflow";
    case "cron":
      return s.kind === "cron";
    default:
      return true;
  }
}
