import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { DataHealthPanel } from "@/components/DataHealthPanel";
import { DeliveryReadinessCard } from "@/components/DeliveryReadinessCard";
import { HomeInsightCards } from "@/components/HomeInsightCards";
import { HomeTokenUsage } from "@/components/HomeTokenUsage";
import { HomeSavedHoursKpi } from "@/components/HomeSavedHoursKpi";
import { HomeTimelineChart } from "@/components/HomeTimelineChart";
import { SecurityActivityPanel } from "@/components/SecurityActivityPanel";
import { SecurityApprovalInbox, PendingApprovalBadge, usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { HomeQuickActions } from "@/components/HomeQuickActions";
import { WorkspacePathsPanel } from "@/components/WorkspacePathsPanel";
import { MetricsChart } from "@/components/MetricsChart";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { PageHeader } from "@/components/ui/PageHeader";
import { WorkbenchStatusCard } from "@/components/WorkbenchStatusCard";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { TrustBar } from "@/components/ui/StatusBadge";
import { useSseStatus } from "@/context/SseContext";
import { SseStatusBadge } from "@/components/SseStatusBadge";
import { useT } from "@/i18n/context";
import { translateBootstrapStep } from "@/i18n/bootstrapTranslate";

export function HomePage() {
  const t = useT();
  const sseStatus = useSseStatus();
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview });
  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const sessions = useQuery({ queryKey: ["all-sessions"], queryFn: () => api.allSessions() });
  const recent = useQuery({ queryKey: ["recent-events"], queryFn: api.recentEvents });
  const running = useQuery({
    queryKey: ["running-sessions"],
    queryFn: api.runningSessions,
    refetchInterval: 3_000,
  });
  const dataHealth = useQuery({ queryKey: ["data-health"], queryFn: api.dataHealth });
  const readiness = useQuery({ queryKey: ["delivery-readiness"], queryFn: api.deliveryReadiness });
  const timeline = useQuery({ queryKey: ["timeline-metrics"], queryFn: () => api.timelineMetrics(7) });
  const bootstrap = useQuery({ queryKey: ["bootstrap"], queryFn: api.bootstrap });
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

  return (
    <>
      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />

      <PageHeader
        title={t("home.title")}
        meta={
          <>
            <span>{t("layout.localMode")}</span>
            <span className="text-outline-variant">•</span>
            <span className="truncate max-w-xs">{t("home.dbLabel")}: {health.data?.db_path ?? "…"}</span>
            <span className="text-outline-variant">•</span>
            <span>v{health.data?.version ?? "…"}</span>
            <span className="text-outline-variant">•</span>
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
          </>
        }
      />

      {ov && ov.sessions_blocked > 0 && (
        <div className="dw-alert-error text-sm">
          {t("home.blockedAlert").replace("{n}", String(ov.sessions_blocked))}{" "}
          <Link to="/conversations" search={{ filter: "blocked" }} className="underline">
            {t("home.blockedAlertAction")}
          </Link>
        </div>
      )}

      {pendingTotal > 0 && (
        <div className="rounded-lg border border-warn/30 bg-warn/10 text-sm px-4 py-3 mb-4">
          {t("home.pendingApprovalAlert").replace("{n}", String(pendingTotal))}{" "}
          <Link to="/conversations" search={{ filter: "needs_approval" }} className="underline">
            {t("home.pendingApprovalAction")}
          </Link>
        </div>
      )}

      <DataHealthPanel health={dataHealth.data?.health} compact />
      <DeliveryReadinessCard readiness={readiness.data?.readiness} compact />
      <HomeQuickActions onNewProject={() => setNewProjectOpen(true)} />
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

      {ov && (
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-7 gap-4">
          <StatCard label={t("home.stats.projects")} value={ov.projects_count} to="/projects" primary />
          <StatCard label={t("home.stats.sessions")} value={ov.sessions_total} to="/conversations" />
          <StatCard
            label={t("home.stats.running")}
            value={ov.sessions_running}
            highlight={ov.sessions_running > 0}
            to="/conversations"
            search={{ filter: "running" }}
          />
          <StatCard
            label={t("home.stats.blocked")}
            value={ov.sessions_blocked}
            danger={ov.sessions_blocked > 0}
            to="/conversations"
            search={{ filter: "blocked" }}
          />
          <StatCard label={t("home.stats.failedGates")} value={ov.gates_failed} danger={ov.gates_failed > 0} />
          <StatCard label={t("home.stats.skills")} value={ov.skills_count} to="/agents" />
          <StatCard label={t("home.stats.events1h")} value={ov.events_last_hour} />
        </div>
      )}

      <HomeInsightCards overview={ov} readiness={readiness.data?.readiness} />
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <HomeTokenUsage />
        <HomeSavedHoursKpi />
      </div>
      <SecurityApprovalInbox />
      <SecurityActivityPanel />

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-5">
          <SectionCard title={t("home.timeline7d")} noPadding>
            <HomeTimelineChart timeline={timeline.data?.timeline} />
          </SectionCard>
        </div>
        <div className="lg:col-span-7">
          <SectionCard title={t("home.projectMetrics")} noPadding>
            <div className="p-4">
              <MetricsChart projects={list} />
            </div>
          </SectionCard>
        </div>
      </div>

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
                            className={`w-2 h-2 rounded-full ${p.trust_score >= 0.9 ? "bg-success" : p.trust_score >= 0.7 ? "bg-warn" : "bg-outline"}`}
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
      </div>

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
                {(running.data?.sessions ?? []).map((s) => (
                  <tr key={s.id}>
                    <td>{s.project_name}</td>
                    <td>
                      <Link to="/sessions/$sessionId" params={{ sessionId: s.id }}>
                        {s.title}
                        <PendingApprovalBadge
                          sessionId={s.id}
                          count={pendingCounts.get(s.id)}
                        />
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

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-7">
          <SectionCard title={t("home.recentSessions")} noPadding>
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
                  {(sessions.data?.sessions ?? []).slice(0, 8).map((s) => (
                    <tr key={s.id}>
                      <td className="text-secondary">{s.project_name}</td>
                      <td>
                        <Link to="/sessions/$sessionId" params={{ sessionId: s.id }}>
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
              {(recent.data?.events ?? []).slice(0, 8).map((e, i, arr) => (
                <div key={e.id} className="dw-timeline-item">
                  {i < arr.length - 1 && <div className="dw-timeline-line" />}
                  <div className="dw-timeline-node info" />
                  <div className="min-w-0">
                    <Link to="/events/$eventId" params={{ eventId: e.id }} className="text-sm font-medium">
                      {e.title}
                    </Link>
                    <p className="text-xs text-secondary m-0 mt-0.5">
                      {e.project_name} · {e.event_type}
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
    </>
  );
}

function StatCard({
  label,
  value,
  to,
  search,
  primary,
  highlight,
  danger,
}: {
  label: string;
  value: number;
  to?: string;
  search?: { filter?: "all" | "running" | "blocked" | "workflow" | "cron" | "needs_approval" };
  primary?: boolean;
  highlight?: boolean;
  danger?: boolean;
}) {
  const inner = (
    <div className="dw-stat-card">
      <span className="dw-stat-label">{label}</span>
      <span
        className={`dw-stat-value ${primary ? "text-primary" : ""} ${danger ? "text-error" : ""} ${highlight ? "text-success" : ""}`}
      >
        {value}
      </span>
    </div>
  );
  if (to) {
    return (
      <Link to={to} search={search} className="no-underline hover:opacity-90">
        {inner}
      </Link>
    );
  }
  return inner;
}
