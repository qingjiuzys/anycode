import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { GitHubIssuesPanel } from "@/components/GitHubIssuesPanel";
import { LinearIssuesPanel } from "@/components/LinearIssuesPanel";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import type { AutomationPolicyRecord } from "@/api/types";
import { useT } from "@/i18n/context";

const POLICY_TYPES = ["gate_block", "cron_notify", "report_on_complete"] as const;

export function AutomationsPage() {
  const t = useT();
  const queryClient = useQueryClient();
  const [projectId, setProjectId] = useState("");
  const [policyName, setPolicyName] = useState("");
  const [policyType, setPolicyType] = useState("gate_block");
  const [policyEnabled, setPolicyEnabled] = useState(true);

  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const workflow = useQuery({
    queryKey: ["automation-sessions", "workflow"],
    queryFn: () => api.sessionsByKind("workflow", 40),
  });
  const cronSessions = useQuery({
    queryKey: ["automation-sessions", "cron"],
    queryFn: () => api.sessionsByKind("cron", 40),
  });
  const cronJobs = useQuery({
    queryKey: ["cron-jobs"],
    queryFn: api.cronJobs,
  });
  const cronRuns = useQuery({
    queryKey: ["cron-runs"],
    queryFn: () => api.cronRuns(30),
  });
  const policies = useQuery({
    queryKey: ["automation-policies", projectId],
    queryFn: () => api.automationPolicies(projectId),
    enabled: Boolean(projectId),
  });
  const connectors = useQuery({
    queryKey: ["connectors", projectId],
    queryFn: () => api.connectors(projectId),
    enabled: Boolean(projectId),
  });

  const upsertPolicy = useMutation({
    mutationFn: () =>
      api.upsertAutomationPolicy(projectId, {
        name: policyName || t("automations.defaultPolicyName"),
        policy_type: policyType,
        config: { risk: "medium" },
        enabled: policyEnabled,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["automation-policies", projectId] });
      setPolicyName("");
    },
  });

  const deletePolicy = useMutation({
    mutationFn: (id: string) => api.deleteAutomationPolicy(projectId, id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["automation-policies", projectId] }),
  });

  const togglePolicy = useMutation({
    mutationFn: (p: AutomationPolicyRecord) =>
      api.upsertAutomationPolicy(projectId, {
        id: p.id,
        name: p.name,
        policy_type: p.policy_type,
        config: p.config as Record<string, unknown>,
        enabled: !p.enabled,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["automation-policies", projectId] });
      queryClient.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const projectList = projects.data?.projects ?? [];
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

      <SectionCard title={t("automations.projectPolicies")}>
        <div className="flex flex-wrap items-center gap-2 mb-3">
          <select
            className="dw-input"
            value={projectId}
            onChange={(e) => setProjectId(e.target.value)}
          >
            <option value="">{t("automations.selectProject")}</option>
            {projectList.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <input
            className="dw-input"
            placeholder={t("automations.policyName")}
            value={policyName}
            onChange={(e) => setPolicyName(e.target.value)}
            disabled={!projectId}
          />
          <select
            className="dw-input"
            value={policyType}
            onChange={(e) => setPolicyType(e.target.value)}
            disabled={!projectId}
          >
            {POLICY_TYPES.map((pt) => (
              <option key={pt} value={pt}>
                {t(`automations.policyTypes.${pt}`)}
              </option>
            ))}
          </select>
          <label className="text-sm text-secondary inline-flex items-center gap-1">
            <input
              type="checkbox"
              checked={policyEnabled}
              onChange={(e) => setPolicyEnabled(e.target.checked)}
              disabled={!projectId}
              className="accent-primary"
            />
            {t("common.enabled")}
          </label>
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={!projectId || upsertPolicy.isPending}
            onClick={() => upsertPolicy.mutate()}
          >
            {upsertPolicy.isPending ? t("automations.saving") : t("automations.addPolicy")}
          </button>
        </div>
        <p className="text-sm text-secondary m-0 mb-3">{t("automations.policyHint")}</p>
        <p className="text-sm text-secondary m-0 mb-3">{t("automations.policyRuntimeHint")}</p>
        {projectId && (
          <PolicyTable
            policies={policies.data?.policies ?? []}
            loading={policies.isLoading}
            onDelete={(id) => deletePolicy.mutate(id)}
            onToggle={(p) => togglePolicy.mutate(p)}
            deleting={deletePolicy.isPending}
            toggling={togglePolicy.isPending}
          />
        )}
      </SectionCard>

      {projectId &&
        githubConnectors.map((c) => (
          <GitHubIssuesPanel
            key={c.id}
            connectorId={c.id}
            connectorName={c.name}
            repo={c.config_summary}
          />
        ))}
      {projectId &&
        linearConnectors.map((c) => (
          <LinearIssuesPanel
            key={c.id}
            connectorId={c.id}
            connectorName={c.name}
            team={c.config_summary}
          />
        ))}

      <SectionCard title={t("automations.cronJobs")} noPadding>
        <div className="overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("common.id")}</th>
                <th>{t("automations.schedule")}</th>
                <th>{t("automations.sessionCol")}</th>
                <th>{t("automations.commandSummary")}</th>
              </tr>
            </thead>
            <tbody>
              {(cronJobs.data?.jobs ?? []).map((j) => (
                <tr key={j.id}>
                  <td>
                    <code className="font-code">{j.id}</code>
                  </td>
                  <td className="text-secondary text-xs">{j.schedule}</td>
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
                  <td>{truncate(j.command, 60)}</td>
                </tr>
              ))}
              {!cronJobs.isLoading && (cronJobs.data?.jobs ?? []).length === 0 && (
                <tr>
                  <td colSpan={4} className="text-secondary text-center py-6">
                    {t("automations.noCronJobs")}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </SectionCard>

      <SectionCard title={t("automations.cronRuns")} noPadding>
        <div className="overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("automations.job")}</th>
                <th>{t("common.status")}</th>
                <th>{t("automations.time")}</th>
                <th>{t("automations.workbenchSession")}</th>
                <th>{t("automations.detail")}</th>
              </tr>
            </thead>
            <tbody>
              {(cronRuns.data?.runs ?? []).map((r) => (
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
                      <Link
                        to="/sessions/$sessionId"
                        params={{ sessionId: r.dashboard_session_id }}
                      >
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
                  <td className="text-secondary text-xs">{truncate(r.detail, 80)}</td>
                </tr>
              ))}
              {!cronRuns.isLoading && (cronRuns.data?.runs ?? []).length === 0 && (
                <tr>
                  <td colSpan={5} className="text-secondary text-center py-6">
                    {t("automations.noCronRuns")}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </SectionCard>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SessionTable
          title={t("automations.workflowSessions")}
          loading={workflow.isLoading}
          sessions={workflow.data?.sessions ?? []}
        />
        <SessionTable
          title={t("automations.cronSessions")}
          loading={cronSessions.isLoading}
          sessions={cronSessions.data?.sessions ?? []}
        />
      </div>
    </>
  );
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

function SessionTable({
  title,
  loading,
  sessions,
}: {
  title: string;
  loading: boolean;
  sessions: import("@/api/types").SessionWithProject[];
}) {
  const t = useT();
  return (
    <SectionCard title={title} noPadding>
      {loading && <p className="text-sm text-secondary px-4 py-3 m-0">{t("common.loading")}</p>}
      <div className="overflow-x-auto">
        <table className="dw-table">
          <thead>
            <tr>
              <th>{t("conversations.project")}</th>
              <th>{t("conversations.titleCol")}</th>
              <th>{t("common.status")}</th>
              <th>{t("conversations.trust")}</th>
            </tr>
          </thead>
          <tbody>
            {sessions.map((s) => (
              <tr key={s.id}>
                <td>
                  <Link
                    to="/projects/$projectId"
                    params={{ projectId: s.project_id }}
                    className="no-underline hover:underline"
                  >
                    {s.project_name}
                  </Link>
                </td>
                <td>
                  <Link
                    to="/sessions/$sessionId"
                    params={{ sessionId: s.id }}
                    className="font-medium no-underline hover:underline"
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
            {!loading && sessions.length === 0 && (
              <tr>
                <td colSpan={4} className="text-secondary text-center py-6">
                  {t("automations.noRecords")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}
