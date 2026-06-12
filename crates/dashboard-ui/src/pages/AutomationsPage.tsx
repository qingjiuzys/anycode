import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { GitHubIssuesPanel } from "@/components/GitHubIssuesPanel";
import { AutomationCreatePanel } from "@/components/AutomationCreatePanel";
import { OrchestrationTasksPanel } from "@/components/OrchestrationTasksPanel";
import { LinearIssuesPanel } from "@/components/LinearIssuesPanel";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { DataTable, DataTableEmpty } from "@/components/ui/DataTable";
import type { AutomationPolicyRecord, CronJobRecord, CronRunRecord } from "@/api/types";
import { useT } from "@/i18n/context";

const POLICY_TYPES = ["gate_block", "cron_notify", "report_on_complete"] as const;

export function AutomationsPage() {
  const t = useT();
  const queryClient = useQueryClient();
  const [projectId, setProjectId] = useState("");
  const [cronProjectFilter, setCronProjectFilter] = useState("");
  const [policyName, setPolicyName] = useState("");
  const [policyType, setPolicyType] = useState("gate_block");
  const [policyEnabled, setPolicyEnabled] = useState(true);

  const projects = useQuery({ queryKey: ["projects"], queryFn: () => api.projects({ limit: 500 }) });
  const cronJobs = useQuery({
    queryKey: ["cron-jobs"],
    queryFn: api.cronJobs,
  });
  const cronRuns = useQuery({
    queryKey: ["cron-runs"],
    queryFn: () => api.cronRuns(30),
  });
  const retryCron = useMutation({
    mutationFn: (body: { job_id: string; project_id?: string }) => api.retryCronJob(body),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["cron-runs"] }),
  });
  const projectList = projects.data?.projects ?? [];
  const selectedProjectId = projectId || projectList[0]?.id || "";
  const selectedProject = projectList.find((p) => p.id === selectedProjectId);
  const policies = useQuery({
    queryKey: ["automation-policies", selectedProjectId],
    queryFn: () => api.automationPolicies(selectedProjectId),
    enabled: Boolean(selectedProjectId),
  });
  const connectors = useQuery({
    queryKey: ["connectors", selectedProjectId],
    queryFn: () => api.connectors(selectedProjectId),
    enabled: Boolean(selectedProjectId),
  });

  const upsertPolicy = useMutation({
    mutationFn: () =>
      api.upsertAutomationPolicy(selectedProjectId, {
        name: policyName || t("automations.defaultPolicyName"),
        policy_type: policyType,
        config: { risk: "medium" },
        enabled: policyEnabled,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["automation-policies", selectedProjectId] });
      setPolicyName("");
    },
  });

  const deletePolicy = useMutation({
    mutationFn: (id: string) => api.deleteAutomationPolicy(selectedProjectId, id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["automation-policies", selectedProjectId] }),
  });

  const togglePolicy = useMutation({
    mutationFn: (p: AutomationPolicyRecord) =>
      api.upsertAutomationPolicy(selectedProjectId, {
        id: p.id,
        name: p.name,
        policy_type: p.policy_type,
        config: p.config as Record<string, unknown>,
        enabled: !p.enabled,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["automation-policies", selectedProjectId] });
      queryClient.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const cronJobList = cronJobs.data?.jobs ?? [];
  const projectNameById = new Map(projectList.map((p) => [p.id, p.name]));
  const filteredCronJobs =
    cronProjectFilter === ""
      ? cronJobList
      : cronProjectFilter === "__workspace__"
        ? cronJobList.filter((j) => !j.project_id)
        : cronJobList.filter((j) => j.project_id === cronProjectFilter);
  const cronRunList = cronRuns.data?.runs ?? [];
  const policyList = policies.data?.policies ?? [];
  const failedRuns = cronRunList.filter((r) => r.status === "failed" || r.status === "error").length;
  const lastRun = cronRunList[0];
  const githubConnectors = (connectors.data?.connectors ?? []).filter(
    (c) => c.enabled && c.source_type === "github" && c.config_summary,
  );
  const linearConnectors = (connectors.data?.connectors ?? []).filter(
    (c) => c.enabled && c.source_type === "linear" && c.config_summary,
  );

  return (
    <>
      <PageHeader
        title={t("automations.title")}
        subtitle={t("automations.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("automations.title") },
        ]}
      />

      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <AutomationMetric
          label={t("automations.activeJobs")}
          value={cronJobs.isLoading ? "…" : String(cronJobList.length)}
          hint={t("automations.activeJobsHint")}
        />
        <AutomationMetric
          label={t("automations.failedRuns")}
          value={cronRuns.isLoading ? "…" : String(failedRuns)}
          hint={t("automations.failedRunsHint")}
          tone={failedRuns > 0 ? "danger" : "success"}
        />
        <AutomationMetric
          label={t("automations.enabledPolicies")}
          value={policies.isLoading ? "…" : String(policyList.filter((p) => p.enabled).length)}
          hint={selectedProject?.name ?? t("automations.noProjectSelected")}
        />
        <AutomationMetric
          label={t("automations.lastRun")}
          value={lastRun ? t(`automations.runStatus.${runStatusKey(lastRun.status)}`) : "—"}
          hint={lastRun?.fired_at ?? t("automations.noRunsYet")}
          tone={lastRun && isRunFailure(lastRun) ? "danger" : "neutral"}
        />
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-[minmax(0,1.05fr)_minmax(360px,0.95fr)] gap-6">
        <AutomationCreatePanel />

        <SectionCard title={t("automations.projectPolicies")}>
          <div className="flex flex-col gap-3">
            <div>
              <label className="block text-xs text-secondary mb-1">{t("automations.policyProject")}</label>
              <select
                className="dw-input w-full"
                value={selectedProjectId}
                onChange={(e) => setProjectId(e.target.value)}
              >
                {!selectedProjectId && <option value="">{t("automations.selectProject")}</option>}
                {projectList.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-secondary mb-1">{t("automations.policyType")}</label>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                {POLICY_TYPES.map((pt) => (
                  <button
                    key={pt}
                    type="button"
                    className={`dw-btn-secondary text-left justify-start ${policyType === pt ? "border-primary text-primary" : ""}`}
                    disabled={!selectedProjectId}
                    onClick={() => setPolicyType(pt)}
                  >
                    {t(`automations.policyTypes.${pt}`)}
                  </button>
                ))}
              </div>
            </div>
            <input
              className="dw-input"
              placeholder={t("automations.policyName")}
              value={policyName}
              onChange={(e) => setPolicyName(e.target.value)}
              disabled={!selectedProjectId}
            />
            <div className="flex flex-wrap items-center gap-3">
              <label className="text-sm text-secondary inline-flex items-center gap-1">
                <input
                  type="checkbox"
                  checked={policyEnabled}
                  onChange={(e) => setPolicyEnabled(e.target.checked)}
                  disabled={!selectedProjectId}
                  className="accent-primary"
                />
                {t("common.enabled")}
              </label>
              <button
                type="button"
                className="dw-btn-primary"
                disabled={!selectedProjectId || upsertPolicy.isPending}
                onClick={() => upsertPolicy.mutate()}
              >
                {upsertPolicy.isPending ? t("automations.saving") : t("automations.addPolicy")}
              </button>
            </div>
          </div>
          <p className="text-sm text-secondary m-0 mt-4">{t("automations.policyRuntimeHint")}</p>
          {selectedProjectId && (
            <PolicyTable
              policies={policyList}
              loading={policies.isLoading}
              onDelete={(id) => deletePolicy.mutate(id)}
              onToggle={(p) => togglePolicy.mutate(p)}
              deleting={deletePolicy.isPending}
              toggling={togglePolicy.isPending}
            />
          )}
        </SectionCard>
      </div>

      {(githubConnectors.length > 0 || linearConnectors.length > 0) && (
        <SectionCard title={t("automations.connectedQueues")}>
          <p className="text-sm text-secondary m-0 mb-3">{t("automations.connectedQueuesHint")}</p>
          <div className="grid grid-cols-1 gap-4">
            {githubConnectors.map((c) => (
              <GitHubIssuesPanel
                key={c.id}
                connectorId={c.id}
                connectorName={c.name}
                repo={c.config_summary}
              />
            ))}
            {linearConnectors.map((c) => (
              <LinearIssuesPanel
                key={c.id}
                connectorId={c.id}
                connectorName={c.name}
                team={c.config_summary}
              />
            ))}
          </div>
        </SectionCard>
      )}

      <SectionCard
        title={t("automations.cronJobs")}
        noPadding
        action={
          <select
            className="dw-input text-xs"
            value={cronProjectFilter}
            onChange={(e) => setCronProjectFilter(e.target.value)}
            aria-label={t("automations.jobProject")}
          >
            <option value="">{t("automations.allProjects")}</option>
            <option value="__workspace__">{t("automations.wholeWorkspace")}</option>
            {projectList.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        }
      >
        <CronJobsTable
          jobs={filteredCronJobs}
          loading={cronJobs.isLoading}
          projectNameById={projectNameById}
          orchestrationPath={cronJobs.data?.orchestration_path}
        />
      </SectionCard>

      <SectionCard title={t("automations.cronRuns")} noPadding>
        <CronRunsTable
          runs={cronRunList}
          loading={cronRuns.isLoading}
          retrying={retryCron.isPending}
          onRetry={(jobId) =>
            retryCron.mutate({
              job_id: jobId,
              project_id: selectedProjectId || undefined,
            })
          }
        />
      </SectionCard>

      <OrchestrationTasksPanel />
    </>
  );
}

function AutomationMetric({
  label,
  value,
  hint,
  tone = "neutral",
}: {
  label: string;
  value: string;
  hint: string;
  tone?: "neutral" | "success" | "danger";
}) {
  const toneClass =
    tone === "danger" ? "text-error" : tone === "success" ? "text-success" : "text-on-surface";
  return (
    <SectionCard className="h-full">
      <p className="text-xs uppercase tracking-wide text-secondary m-0 mb-2">{label}</p>
      <p className={`text-2xl font-semibold m-0 ${toneClass}`}>{value}</p>
      <p className="text-xs text-secondary m-0 mt-2 truncate" title={hint}>
        {hint}
      </p>
    </SectionCard>
  );
}

function CronJobsTable({
  jobs,
  loading,
  projectNameById,
  orchestrationPath,
}: {
  jobs: CronJobRecord[];
  loading: boolean;
  projectNameById: Map<string, string>;
  orchestrationPath?: string;
}) {
  const t = useT();
  if (loading) return <p className="text-sm text-secondary px-4 py-6 m-0">{t("common.loading")}</p>;
  return (
    <DataTable
      isEmpty={jobs.length === 0}
      empty={
        <div className="flex flex-col items-center gap-1">
          <p className="text-sm text-secondary m-0">{t("automations.noCronJobs")}</p>
          {orchestrationPath && (
            <p className="text-xs text-secondary m-0">
              {t("automations.orchestrationFileLabel")}:{" "}
              <code className="font-code break-all">{orchestrationPath}</code>
            </p>
          )}
          <p className="text-xs text-secondary m-0">{t("automations.registerHint")}</p>
        </div>
      }
    >
      <thead>
        <tr>
          <th>{t("common.id")}</th>
          <th>{t("automations.schedule")}</th>
          <th>{t("automations.jobProject")}</th>
          <th>{t("automations.sessionCol")}</th>
          <th>{t("automations.toolProfile")}</th>
          <th>{t("automations.commandSummary")}</th>
        </tr>
      </thead>
      <tbody>
        {jobs.map((j) => (
          <tr key={j.id}>
            <td>
              <code className="font-code">{j.id}</code>
            </td>
            <td className="text-secondary text-xs">{j.schedule}</td>
            <td className="text-xs">
              {j.project_id ? (
                <span className="text-on-surface">
                  {projectNameById.get(j.project_id) ?? j.project_id}
                </span>
              ) : (
                <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-surface-container-high text-secondary">
                  {t("automations.wholeWorkspace")}
                </span>
              )}
            </td>
            <td className="text-secondary font-code text-xs">
              {j.session_id ? (
                <Link
                  to="/sessions/$sessionId"
                  params={{ sessionId: j.session_id }}
                  className="no-underline hover:underline"
                >
                  {j.session_id.slice(0, 12)}…
                </Link>
              ) : (
                "—"
              )}
            </td>
            <td className="text-secondary text-xs">{j.tool_profile ?? "—"}</td>
            <td>{truncate(j.command, 72)}</td>
          </tr>
        ))}
      </tbody>
    </DataTable>
  );
}

function CronRunsTable({
  runs,
  loading,
  retrying,
  onRetry,
}: {
  runs: CronRunRecord[];
  loading: boolean;
  retrying: boolean;
  onRetry: (jobId: string) => void;
}) {
  const t = useT();
  if (loading) return <p className="text-sm text-secondary px-4 py-6 m-0">{t("common.loading")}</p>;
  return (
    <DataTable
      isEmpty={runs.length === 0}
      empty={<DataTableEmpty message={t("automations.noCronRuns")} />}
    >
      <thead>
        <tr>
          <th>{t("automations.job")}</th>
          <th>{t("common.status")}</th>
          <th>{t("automations.time")}</th>
          <th>{t("automations.workbenchSession")}</th>
          <th>{t("automations.detail")}</th>
          <th>{t("common.actions")}</th>
        </tr>
      </thead>
      <tbody>
        {runs.map((r) => (
          <tr key={`${r.line_no}-${r.fired_at}`}>
            <td>
              <code className="font-code">{r.job_id}</code>
            </td>
            <td>
              <CronRunBadge status={r.status} />
            </td>
            <td className="text-secondary text-xs">{r.fired_at}</td>
            <td>
              {r.dashboard_session_id ? (
                <Link to="/sessions/$sessionId" params={{ sessionId: r.dashboard_session_id }}>
                  {t("automations.view")}
                </Link>
              ) : r.session_id ? (
                <span className="text-secondary font-code text-xs" title="correlation id">
                  {r.session_id.slice(0, 8)}…
                </span>
              ) : (
                "—"
              )}
            </td>
            <td className="text-secondary text-xs">{truncate(r.detail, 90)}</td>
            <td>
              {isRunFailure(r) && (
                <button
                  type="button"
                  className="dw-btn-secondary text-xs"
                  disabled={retrying}
                  onClick={() => onRetry(r.job_id)}
                >
                  {t("automations.retryRun")}
                </button>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </DataTable>
  );
}

function isRunFailure(r: CronRunRecord): boolean {
  return r.status === "failed" || r.status === "error";
}

function runStatusKey(status: string): "ok" | "error" | "started" | "other" {
  if (status === "ok") return "ok";
  if (status === "error" || status === "failed") return "error";
  if (status === "started") return "started";
  return "other";
}

function PolicyTable({
  policies,
  loading,
  onDelete,
  onToggle,
  deleting,
  toggling,
}: {
  policies: AutomationPolicyRecord[];
  loading: boolean;
  onDelete: (id: string) => void;
  onToggle: (p: AutomationPolicyRecord) => void;
  deleting: boolean;
  toggling: boolean;
}) {
  const t = useT();
  if (loading) return <p className="text-sm text-secondary">{t("automations.loadingPolicies")}</p>;
  if (policies.length === 0) {
    return <p className="text-sm text-secondary">{t("automations.noPolicies")}</p>;
  }
  return (
    <div className="overflow-x-auto -mx-4 px-4">
      <table className="dw-table">
        <thead>
          <tr>
            <th>{t("common.name")}</th>
            <th>{t("conversations.type")}</th>
            <th>{t("common.status")}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {policies.map((p) => (
            <tr key={p.id}>
              <td>{p.name}</td>
              <td>
                {POLICY_TYPES.includes(p.policy_type as (typeof POLICY_TYPES)[number]) ? (
                  t(`automations.policyTypes.${p.policy_type as (typeof POLICY_TYPES)[number]}`)
                ) : (
                  <code className="font-code">{p.policy_type}</code>
                )}
              </td>
              <td>{p.enabled ? t("common.enabled") : t("common.disabled")}</td>
              <td className="text-right whitespace-nowrap">
                <button
                  type="button"
                  className="dw-btn-ghost text-xs"
                  disabled={toggling}
                  onClick={() => onToggle(p)}
                >
                  {p.enabled ? t("common.disable") : t("common.enable")}
                </button>
                <button
                  type="button"
                  className="dw-btn-ghost text-xs"
                  disabled={deleting}
                  onClick={() => onDelete(p.id)}
                >
                  {t("common.delete")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function CronRunBadge({ status }: { status: string }) {
  const mapped =
    status === "ok" ? "ok" : status === "error" ? "error" : status === "started" ? "running" : status;
  return <StatusBadge status={mapped} />;
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return `${s.slice(0, max)}…`;
}
