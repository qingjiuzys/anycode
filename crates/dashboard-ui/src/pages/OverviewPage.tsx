import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { buildConversationsHref } from "@/lib/conversationsSearch";
import { api } from "@/api/client";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { HomeInsightCards } from "@/components/HomeInsightCards";
import { HomeWorkbenchPanel } from "@/components/HomeWorkbenchPanel";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { HomeTokenUsage } from "@/components/HomeTokenUsage";
import { HomeSavedHoursKpi } from "@/components/HomeSavedHoursKpi";
import { HomeTimelineChart } from "@/components/HomeTimelineChart";
import { SecurityActivityPanel } from "@/components/SecurityActivityPanel";
import {
  SecurityApprovalInbox,
  PendingApprovalBadge,
  usePendingApprovalCounts,
} from "@/components/SecurityApprovalInbox";
import { HomeOverviewPanelPills } from "@/components/HomeOverviewPanelPills";
import { HomePanelOverlays, type HomePanelSection } from "@/components/HomePanelOverlays";
import { WorkspacePathsPanel } from "@/components/WorkspacePathsPanel";
import { MetricsChart } from "@/components/MetricsChart";
import { WorkbenchStatusCard } from "@/components/WorkbenchStatusCard";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { TrustBar } from "@/components/ui/StatusBadge";
import { useSseStatus } from "@/context/SseContext";
import { SseStatusBadge } from "@/components/SseStatusBadge";
import { useT } from "@/i18n/context";
import { translateBootstrapStep } from "@/i18n/bootstrapTranslate";
import { sessionChatSearch } from "@/lib/sessionLinks";
import { formatEventTitle, formatEventTypeLabel } from "@/lib/eventFormat";
import type { EmbeddedPageProps } from "@/lib/pageProps";

export function OverviewPage(_props: EmbeddedPageProps = {}) {
  const t = useT();
  const sseStatus = useSseStatus();
  const [expandedSection, setExpandedSection] = useState<string | null>(null);
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const analyticsOpen = expandedSection === "analytics";
  const workbenchOpen = expandedSection === "workbench";
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview });
  const projects = useQuery({
    queryKey: ["projects", "home-top"],
    queryFn: () => api.projects({ limit: 8, sort: "updated_at_desc" }),
  });
  const sessions = useQuery({
    queryKey: ["all-sessions", "home-recent"],
    queryFn: () => api.allSessions({ limit: 8 }),
  });
  const recent = useQuery({ queryKey: ["recent-events"], queryFn: api.recentEvents });
  const running = useQuery({
    queryKey: ["running-sessions"],
    queryFn: api.runningSessions,
    refetchInterval: 3_000,
  });
  const readiness = useQuery({ queryKey: ["delivery-readiness"], queryFn: api.deliveryReadiness });
  const timeline = useQuery({
    queryKey: ["timeline-metrics"],
    queryFn: () => api.timelineMetrics(7),
    enabled: analyticsOpen,
  });
  const bootstrap = useQuery({
    queryKey: ["bootstrap"],
    queryFn: api.bootstrap,
    enabled: workbenchOpen,
  });
  const { counts: pendingCounts, pendingTotal } = usePendingApprovalCounts();

  if (health.isError) {
    return (
      <div className="dw-alert-error">
        {t("home.apiError")} <code className="font-code">anycode dashboard</code>
      </div>
    );
  }

  const list = projects.data?.projects ?? [];
  const ov = overview.data?.overview;
  const steps = bootstrap.data?.bootstrap?.next_steps ?? [];
  const recentSessions = sessions.data?.sessions ?? [];

  const analyticsContent = (
    <div className="dw-analytics-stack">
      <HomeTokenUsage />
      <HomeSavedHoursKpi />
      <SecurityActivityPanel variant="analytics" />
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <SectionCard title={t("home.timeline7d")} noPadding className="dw-analytics-chart-card">
          <HomeTimelineChart timeline={timeline.data?.timeline} tall />
        </SectionCard>
        <SectionCard title={t("home.projectMetrics")} noPadding className="dw-analytics-chart-card">
          <div className="px-2 pb-2 pt-1">
            <MetricsChart projects={list} tall />
          </div>
        </SectionCard>
      </div>
    </div>
  );

  const workbenchContent = (
    <>
      <WorkbenchStatusCard bootstrap={bootstrap.data?.bootstrap} />
      <WorkspacePathsPanel bootstrap={bootstrap.data?.bootstrap} />
      {steps.length > 0 && (
        <SectionCard title={t("home.nextSteps")}>
          <ul className="m-0 pl-5 text-sm text-secondary space-y-1">
            {steps.map((step) => (
              <li key={step}>{translateBootstrapStep(t, step)}</li>
            ))}
          </ul>
        </SectionCard>
      )}
      <SecurityApprovalInbox />
    </>
  );

  const moreSections: HomePanelSection[] = [
    ...(recentSessions.length > 0
      ? [
          {
            id: "recent",
            title: t("home.recentSessions"),
            content: (
              <div className="overflow-x-auto">
                <table className="dw-table">
                  <thead>
                    <tr>
                      <th>{t("assets.project")}</th>
                      <th>{t("conversations.titleCol")}</th>
                      <th>{t("common.status")}</th>
                      <th>{t("conversations.trust")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recentSessions.slice(0, 6).map((s) => (
                      <tr key={s.id}>
                        <td className="text-secondary">{s.project_name}</td>
                        <td>
                          <Link
                            to={buildConversationsHref({
                              session: s.id,
                              project: s.project_id,
                            })}
                          >
                            {s.title}
                          </Link>
                        </td>
                        <td>
                          <StatusBadge status={s.status} />
                        </td>
                        <td>
                          <StatusBadge status={s.trusted_status} />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ),
          },
        ]
      : []),
    {
      id: "analytics",
      title: t("home.analyticsSection"),
      content: analyticsContent,
    },
    {
      id: "workbench",
      title: t("home.workbenchSection"),
      content: workbenchContent,
    },
  ];

  return (
    <>
      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />

      <PageHeader
        title={t("nav.overview")}
        subtitle={t("overview.subtitle")}
        breadcrumbs={[{ label: t("breadcrumb.home"), to: "/" }, { label: t("nav.overview") }]}
        actions={
          <HomeOverviewPanelPills
            activePanelId={expandedSection}
            onPanelChange={setExpandedSection}
            showRecentPanel={recentSessions.length > 0}
          />
        }
      />

      <div className="mb-6">
        <h2 className="text-base font-semibold text-on-surface m-0 mb-3">{t("overview.workbenchQuick")}</h2>
        <HomeWorkbenchPanel
          overview={ov}
          projects={list}
          loadingProjects={projects.isLoading}
          pendingApprovals={pendingTotal}
          onNewProject={() => setNewProjectOpen(true)}
        />
      </div>

      {ov && (ov.sessions_blocked > 0 || pendingTotal > 0 || ov.sessions_budget_exceeded > 0) && (
        <SectionCard title={t("home.opsSummary")}>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <StatCard
              label={t("home.stats.blocked")}
              value={ov.sessions_blocked}
              danger={ov.sessions_blocked > 0}
              to={buildConversationsHref({ filter: "blocked" })}
            />
            <StatCard
              label={t("home.pendingApprovals")}
              value={pendingTotal}
              highlight={pendingTotal > 0}
              to={buildConversationsHref({ filter: "needs_approval" })}
            />
            <StatCard
              label={t("home.budgetExceeded")}
              value={ov.sessions_budget_exceeded}
              highlight={ov.sessions_budget_exceeded > 0}
              to={buildConversationsHref({ filter: "budget" })}
            />
          </div>
        </SectionCard>
      )}

      {ov && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <StatCard label={t("home.stats.projects")} value={ov.projects_count} to="/projects" primary />
          <StatCard label={t("home.stats.sessions")} value={ov.sessions_total} to="/conversations" />
          <StatCard
            label={t("home.stats.running")}
            value={ov.sessions_running}
            highlight={ov.sessions_running > 0}
            to={buildConversationsHref({ filter: "running" })}
          />
            <StatCard
              label={t("home.stats.blocked")}
              value={ov.sessions_blocked}
              danger={ov.sessions_blocked > 0}
              to={buildConversationsHref({ filter: "blocked" })}
            />
        </div>
      )}

      <HomeInsightCards
        overview={ov}
        readiness={readiness.data?.readiness}
        firstProjectId={list[0]?.id}
      />

      {(running.data?.sessions ?? []).length > 0 && (
        <SectionCard title={t("home.running")} noPadding>
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("assets.project")}</th>
                  <th>{t("conversations.titleCol")}</th>
                  <th>{t("conversations.type")}</th>
                  <th>{t("conversations.trust")}</th>
                  <th>{t("common.actions")}</th>
                </tr>
              </thead>
              <tbody>
                {(running.data?.sessions ?? []).slice(0, 5).map((s) => (
                  <tr key={s.id}>
                    <td>{s.project_name}</td>
                    <td>
                      <Link
                        to="/conversations"
                        search={sessionChatSearch(s.id, s.project_id)}
                      >
                        {s.title}
                        <PendingApprovalBadge sessionId={s.id} count={pendingCounts.get(s.id)} />
                      </Link>
                    </td>
                    <td className="text-secondary">{s.kind}</td>
                    <td>
                      <StatusBadge status={s.trusted_status} />
                    </td>
                    <td>
                      <CancelSessionButton sessionId={s.id} status={s.status} compact />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </SectionCard>
      )}

      {pendingTotal > 0 && <SecurityApprovalInbox compact hideWhenEmpty />}

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-7">
          <SectionCard
            title={t("home.activeProjects")}
            action={
              <Link to="/projects" className="text-xs text-primary hover:underline">
                {t("common.viewAll")}
              </Link>
            }
            noPadding
          >
            <div className="overflow-x-auto">
              <table className="dw-table">
                <thead>
                  <tr>
                    <th>{t("common.name")}</th>
                    <th className="text-right">{t("projects.sessions")}</th>
                    <th className="text-right">{t("home.trust")}</th>
                  </tr>
                </thead>
                <tbody>
                  {list.map((p) => (
                    <tr key={p.id}>
                      <td>
                        <Link
                          to="/projects/$projectId"
                          params={{ projectId: p.id }}
                          className="font-medium flex items-center gap-2 no-underline hover:underline"
                        >
                          <span
                            className={`w-2 h-2 rounded-full ${
                              p.trust_score == null
                                ? "bg-outline"
                                : p.trust_score >= 0.9
                                  ? "bg-success"
                                  : p.trust_score >= 0.7
                                    ? "bg-warn"
                                    : "bg-error"
                            }`}
                            title={p.trust_score == null ? t("trust.notEvaluated") : undefined}
                          />
                          {p.name}
                        </Link>
                      </td>
                      <td className="text-right font-code">{p.sessions_count}</td>
                      <td className="text-right">
                        <TrustBar score={p.trust_score} />
                      </td>
                    </tr>
                  ))}
                  {list.length === 0 && (
                    <tr>
                      <td colSpan={3} className="text-secondary text-center py-8">
                        {t("home.noProjects")}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </SectionCard>
        </div>
        <div className="lg:col-span-5">
          <SectionCard
            title={t("home.liveEvents")}
            action={
              <SseStatusBadge
                status={
                  sseStatus === "live"
                    ? "live"
                    : sseStatus === "connecting"
                      ? "connecting"
                      : sseStatus === "reconnecting"
                        ? "reconnecting"
                        : "offline"
                }
              />
            }
          >
            <div className="dw-timeline">
              {(recent.data?.events ?? []).slice(0, 6).map((e, i, arr) => (
                <div key={e.id} className="dw-timeline-item">
                  {i < arr.length - 1 && <div className="dw-timeline-line" />}
                  <div className="dw-timeline-node info" />
                  <div className="min-w-0">
                    <Link to="/events/$eventId" params={{ eventId: e.id }} className="text-sm font-medium">
                      {formatEventTitle(e, t)}
                    </Link>
                    <p className="text-xs text-secondary m-0 mt-0.5">
                      {e.project_name} · {formatEventTypeLabel(e.event_type, t)}
                    </p>
                    <time className="text-[10px] text-outline">{e.occurred_at}</time>
                  </div>
                </div>
              ))}
              {(recent.data?.events ?? []).length === 0 && (
                <p className="text-sm text-secondary">{t("home.noEvents")}</p>
              )}
            </div>
          </SectionCard>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2 text-xs font-code text-secondary pt-2 border-t border-outline-variant/40">
        <span>{t("layout.localMode")}</span>
        <span className="text-outline-variant">•</span>
        <span className="truncate max-w-xs">
          {t("home.dbLabel")}: {health.data?.db_path ?? "…"}
        </span>
        <span className="text-outline-variant">•</span>
        <span>v{health.data?.version ?? "…"}</span>
      </div>

      <HomePanelOverlays
        sections={moreSections}
        activeId={expandedSection}
        onActiveChange={setExpandedSection}
      />
    </>
  );
}

function StatCard({
  label,
  value,
  to,
  primary,
  highlight,
  danger,
}: {
  label: string;
  value: number;
  to?: string;
  primary?: boolean;
  highlight?: boolean;
  danger?: boolean;
}) {
  const inner = (
    <div className="dw-stat-card">
      <div className="text-xs text-secondary">{label}</div>
      <div
        className={`text-2xl font-semibold font-code ${
          danger ? "text-error" : highlight ? "text-warn" : primary ? "text-primary" : ""
        }`}
      >
        {value}
      </div>
    </div>
  );
  if (!to) return inner;
  return (
    <Link to={to} className="no-underline">
      {inner}
    </Link>
  );
}
