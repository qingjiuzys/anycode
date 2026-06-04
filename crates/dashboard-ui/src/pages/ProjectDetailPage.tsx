import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { api } from "@/api/client";
import { EventTimeline } from "@/components/EventTimeline";
import { DataHealthPanel } from "@/components/DataHealthPanel";
import { GateRunnerPanel } from "@/components/GateRunnerPanel";
import { Icon } from "@/components/Icon";
import { ProjectInsightCharts } from "@/components/ProjectInsightCharts";
import { ProjectTokenUsage } from "@/components/ProjectTokenUsage";
import { SessionFlow } from "@/components/SessionFlow";
import { PageHeader } from "@/components/ui/PageHeader";
import { ProjectKnowledgePanel } from "@/components/ProjectKnowledgePanel";
import { DataTable, DataTableEmpty } from "@/components/ui/DataTable";
import { KpiMetricGrid } from "@/components/KpiMetricGrid";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useProjectEventStream } from "@/hooks/useProjectEventStream";
import { useT } from "@/i18n/context";

const SEVERITIES = ["info", "warn", "error"] as const;
const TOOL_CALL_FILTER = "tool_call_end";

export function ProjectDetailPage() {
  const t = useT();
  const { projectId } = useParams({ from: "/_shell/projects/$projectId" });
  const queryClient = useQueryClient();
  const [eventFilter, setEventFilter] = useState<string | null>(null);
  const [severityFilter, setSeverityFilter] = useState<string | null>(null);
  const [eventSearch, setEventSearch] = useState("");
  useProjectEventStream(projectId);
  const reindex = useMutation({
    mutationFn: () => api.reindexProject(projectId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      queryClient.invalidateQueries({ queryKey: ["events", projectId] });
      queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      queryClient.invalidateQueries({ queryKey: ["gates", projectId] });
      queryClient.invalidateQueries({ queryKey: ["project-skills", projectId] });
      queryClient.invalidateQueries({ queryKey: ["project-stats", projectId] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
    },
  });
  const project = useQuery({
    queryKey: ["project", projectId],
    queryFn: () => api.project(projectId),
  });
  const stats = useQuery({
    queryKey: ["project-stats", projectId],
    queryFn: () => api.projectStats(projectId),
  });
  const gates = useQuery({
    queryKey: ["gates", projectId],
    queryFn: () => api.gates(projectId),
  });
  const projectEventTypes = useQuery({
    queryKey: ["project-event-types", projectId],
    queryFn: () => api.projectEventTypes(projectId),
  });
  const events = useQuery({
    queryKey: ["events", projectId, eventFilter, severityFilter, eventSearch],
    queryFn: () =>
      api.events(projectId, {
        eventType: eventFilter ?? undefined,
        severity: severityFilter ?? undefined,
        q: eventSearch.trim() || undefined,
      }),
  });
  const sessionsQ = useQuery({
    queryKey: ["sessions", projectId],
    queryFn: () => api.sessions(projectId),
  });
  const skillsQ = useQuery({
    queryKey: ["project-skills", projectId],
    queryFn: () => api.projectSkills(projectId),
  });
  const projectHealth = useQuery({
    queryKey: ["project-data-health", projectId],
    queryFn: () => api.projectDataHealth(projectId),
  });
  const projectMetrics = useQuery({
    queryKey: ["project-metrics", projectId],
    queryFn: () => api.projectMetrics(projectId),
  });
  const indexAssets = useMutation({
    mutationFn: () => api.indexProjectAssets(projectId),
  });
  const toggleSkill = useMutation({
    mutationFn: ({ skillId, enabled }: { skillId: string; enabled: boolean }) =>
      api.setProjectSkill(projectId, skillId, enabled),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["project-skills", projectId] }),
  });

  const p = project.data?.project;
  const blocked = (gates.data?.gates ?? []).some(
    (g) => g.required && g.status === "failed",
  );

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("nav.projects"), to: "/projects" },
          { label: p?.name ?? projectId },
        ]}
        title={p?.name ?? projectId}
        subtitle={p?.root_path}
        meta={
          p ? (
            <>
              {p.description && <span className="line-clamp-1">{p.description}</span>}
              {p.business_goal && (
                <>
                  <span className="text-outline-variant">·</span>
                  <span className="text-secondary">{p.business_goal}</span>
                </>
              )}
              {p.automation_level > 0 && (
                <>
                  <span className="text-outline-variant">·</span>
                  <span>
                    {t("projectDetail.automationLevel")}: {p.automation_level}
                  </span>
                </>
              )}
            </>
          ) : undefined
        }
        actions={
          <>
            <button
              type="button"
              className="dw-btn-secondary"
              disabled={reindex.isPending}
              onClick={() => reindex.mutate()}
            >
              <Icon name="sync" size={16} />
              {reindex.isPending ? t("projectDetail.reindexing") : t("projectDetail.reindex")}
            </button>
            <button
              type="button"
              className="dw-btn-secondary"
              disabled={indexAssets.isPending}
              onClick={() => indexAssets.mutate()}
            >
              <Icon name="inventory" size={16} />
              {indexAssets.isPending ? t("projectDetail.indexing") : t("projectDetail.indexAssets")}
            </button>
            <Link
              to="/reports"
              search={{ project_id: projectId }}
              className="dw-btn-secondary no-underline"
            >
              <Icon name="description" size={16} />
              {t("projectDetail.generateReport")}
            </Link>
          </>
        }
      />

      {reindex.isSuccess && (
        <p className="text-sm text-secondary m-0">
          {t("projectDetail.reindexResult")
            .replace("{skills}", String(reindex.data.skills_synced))}
        </p>
      )}
      {indexAssets.isSuccess && indexAssets.data?.result && (
        <p className="text-sm text-secondary m-0">
          {t("projectDetail.assetsResult")
            .replace("{indexed}", String(indexAssets.data.result.indexed))
            .replace("{missing}", String(indexAssets.data.result.missing))
            .replace("{skipped}", String(indexAssets.data.result.skipped))}
        </p>
      )}

      {blocked && (
        <div className="dw-alert-error">{t("projectDetail.gateBlocked")}</div>
      )}

      <DataHealthPanel health={projectHealth.data?.health} compact />

      <div className="dw-project-detail">
        {projectMetrics.data?.metrics && (
          <KpiMetricGrid
            metrics={[
              {
                label: t("projectDetail.readinessScore"),
                value: String(projectMetrics.data.metrics.readiness_score),
                highlight: true,
              },
              {
                label: t("projectDetail.events7d"),
                value: String(projectMetrics.data.metrics.events_7d),
              },
              {
                label: t("projectDetail.gatePassRate"),
                value: `${(projectMetrics.data.metrics.gate_pass_rate * 100).toFixed(0)}%`,
              },
              {
                label: t("projectDetail.unverifiedAssets"),
                value: String(projectMetrics.data.metrics.unverified_artifacts),
              },
            ]}
          />
        )}

        {stats.data?.stats && <ProjectInsightCharts stats={stats.data.stats} />}

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <SectionCard title={t("projectDetail.sessions")} noPadding>
            <DataTable
              isEmpty={(sessionsQ.data?.sessions ?? []).length === 0}
              empty={
                <DataTableEmpty
                  message={t("projectDetail.emptySessions")}
                  icon={<Icon name="forum" size={28} className="text-outline" />}
                />
              }
            >
              <thead>
                <tr>
                  <th>{t("conversations.titleCol")}</th>
                  <th>{t("conversations.type")}</th>
                  <th>{t("common.status")}</th>
                  <th>{t("conversations.trust")}</th>
                </tr>
              </thead>
              <tbody>
                {(sessionsQ.data?.sessions ?? []).map((s) => (
                  <tr key={s.id}>
                    <td>
                      <Link
                        to="/sessions/$sessionId"
                        params={{ sessionId: s.id }}
                        className="font-medium no-underline hover:underline"
                      >
                        {s.title}
                      </Link>
                    </td>
                    <td className="text-secondary text-xs">{s.kind}</td>
                    <td>
                      <StatusBadge status={s.status} />
                    </td>
                    <td>
                      <StatusBadge status={s.trusted_status} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </DataTable>
          </SectionCard>

          <SectionCard title={t("projectDetail.acceptanceGates")} noPadding>
            <DataTable
              isEmpty={(gates.data?.gates ?? []).length === 0}
              empty={<DataTableEmpty message={t("projectDetail.noGates")} icon={<Icon name="verified" size={28} className="text-outline" />} />}
            >
              <thead>
                <tr>
                  <th>{t("common.name")}</th>
                  <th>{t("common.status")}</th>
                  <th>{t("projectDetail.required")}</th>
                  <th>{t("session.gateOutput")}</th>
                </tr>
              </thead>
              <tbody>
                {(gates.data?.gates ?? []).map((g) => (
                  <tr key={g.id}>
                    <td className="font-medium">{g.name}</td>
                    <td>
                      <StatusBadge status={g.status} />
                    </td>
                    <td className="text-secondary text-xs">
                      {g.required ? t("projectDetail.yes") : t("projectDetail.no")}
                    </td>
                    <td className="text-secondary text-xs max-w-xs">{g.output_excerpt || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </DataTable>
          </SectionCard>
        </div>

        <div className="dw-project-cta-bar">
          <p className="text-sm text-secondary m-0 max-w-xl">{t("projectDetail.startTaskHint")}</p>
          <Link
            to="/conversations"
            search={{ project: projectId }}
            className="dw-btn-primary inline-flex items-center gap-2 no-underline shrink-0"
          >
            <Icon name="forum" size={16} />
            {t("projectDetail.openConversations")}
          </Link>
        </div>

        <div className="dw-project-secondary-grid">
          <ProjectTokenUsage projectId={projectId} />
          <GateRunnerPanel projectId={projectId} />
        </div>

        {(stats.data?.stats?.recent_failures ?? []).length > 0 && (
          <SectionCard title={t("projectDetail.recentFailures")}>
            <ul className="m-0 pl-5 text-sm space-y-2">
              {(stats.data?.stats?.recent_failures ?? []).map((f) => (
                <li key={f.id}>
                  <Link to="/events/$eventId" params={{ eventId: f.id }}>
                    {f.title}
                  </Link>
                  <span className="text-secondary">
                    {" "}
                    · {f.event_type} · {f.occurred_at}
                  </span>
                  {f.session_id && (
                    <>
                      {" "}
                      <Link
                        to="/sessions/$sessionId"
                        params={{ sessionId: f.session_id }}
                        className="text-secondary"
                      >
                        {t("projectDetail.sessionLink")}
                      </Link>
                    </>
                  )}
                </li>
              ))}
            </ul>
          </SectionCard>
        )}

        {(sessionsQ.data?.sessions ?? []).length > 0 && (
          <SectionCard title={t("projectDetail.pipeline")}>
            <p className="text-xs text-secondary m-0 mb-2">{t("sessionFlow.hint")}</p>
            <SessionFlow sessions={sessionsQ.data?.sessions ?? []} />
          </SectionCard>
        )}

        {(skillsQ.data?.skills ?? []).length > 0 && (
          <SectionCard title={t("projectDetail.projectSkills")} noPadding>
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("projectDetail.skillCol")}</th>
                  <th>{t("common.path")}</th>
                  <th>{t("projectDetail.enabled")}</th>
                </tr>
              </thead>
              <tbody>
                {(skillsQ.data?.skills ?? []).map((sk) => (
                  <tr key={sk.id}>
                    <td>
                      <Link
                        to="/agents/$skillId"
                        params={{ skillId: sk.id }}
                        className="font-medium no-underline hover:underline"
                      >
                        {sk.name}
                      </Link>
                    </td>
                    <td className="text-secondary font-code text-xs">{sk.source_path}</td>
                    <td>
                      <input
                        type="checkbox"
                        checked={sk.enabled ?? false}
                        disabled={toggleSkill.isPending}
                        onChange={(e) =>
                          toggleSkill.mutate({
                            skillId: sk.id,
                            enabled: e.target.checked,
                          })
                        }
                        className="accent-primary"
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          </SectionCard>
        )}

        <ProjectKnowledgePanel projectId={projectId} />

        <SectionCard
          title={t("projectDetail.recentEvents")}
        action={
          <input
            type="search"
            className="dw-input w-48"
            placeholder={t("events.searchPlaceholder")}
            value={eventSearch}
            onChange={(e) => setEventSearch(e.target.value)}
          />
        }
        >
          <div className="flex flex-wrap gap-2 mb-2">
          <button
            type="button"
            className={`dw-chip${eventFilter === null ? " active" : ""}`}
            onClick={() => setEventFilter(null)}
          >
            {t("events.allTypes")}
          </button>
          <button
            type="button"
            className={`dw-chip${eventFilter === TOOL_CALL_FILTER ? " active" : ""}`}
            onClick={() =>
              setEventFilter((f) => (f === TOOL_CALL_FILTER ? null : TOOL_CALL_FILTER))
            }
          >
            {t("session.filterToolCalls")}
          </button>
          {(projectEventTypes.data?.event_types ?? [])
            .filter((etype) => !etype.startsWith("tool_call"))
            .map((etype) => (
            <button
              key={etype}
              type="button"
              className={`dw-chip${eventFilter === etype ? " active" : ""}`}
              onClick={() => setEventFilter(etype)}
            >
              {etype}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap gap-2 mb-4">
          <button
            type="button"
            className={`dw-chip${severityFilter === null ? " active" : ""}`}
            onClick={() => setSeverityFilter(null)}
          >
            {t("events.allSeverities")}
          </button>
          {SEVERITIES.map((s) => (
            <button
              key={s}
              type="button"
              className={`dw-chip${severityFilter === s ? " active" : ""}`}
              onClick={() => setSeverityFilter(s)}
            >
              {t(`status.${s}`)}
            </button>
          ))}
        </div>
          <EventTimeline events={events.data?.events ?? []} />
        </SectionCard>
      </div>
    </>
  );
}
