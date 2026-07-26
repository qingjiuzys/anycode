import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { OrchestrationTasksPanel } from "@/components/OrchestrationTasksPanel";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { DataTable, DataTableEmpty } from "@/components/ui/DataTable";
import type { CronRunRecord, DoctorCheck, NotificationPolicyRecord } from "@/api/types";
import { useT } from "@/i18n/context";
import { sessionChatSearch } from "@/lib/sessionLinks";

const PIPELINE_DOCTOR_IDS = ["cron_scheduler"] as const;
const AUTOMATION_NOTIFY_EVENTS = new Set([
  "gate_failed",
  "session_blocked",
  "session_report_generated",
  "project_report_generated",
  "blocked_threshold_exceeded",
]);

export function AutomationsOpsPanel({ className = "" }: { className?: string }) {
  const t = useT();
  const queryClient = useQueryClient();
  const [cronProjectFilter, setCronProjectFilter] = useState("");

  const projects = useQuery({ queryKey: ["projects"], queryFn: () => api.projects({ limit: 500 }) });
  const cronJobs = useQuery({ queryKey: ["cron-jobs"], queryFn: api.cronJobs });
  const cronRuns = useQuery({
    queryKey: ["cron-runs"],
    queryFn: () => api.cronRuns(30),
    refetchInterval: 30_000,
  });
  const doctor = useQuery({ queryKey: ["doctor"], queryFn: api.doctor });
  const notificationPolicies = useQuery({
    queryKey: ["notifications"],
    queryFn: () => api.notificationPolicies(),
  });

  const retryCron = useMutation({
    mutationFn: (body: { job_id: string; project_id?: string }) => api.retryCronJob(body),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["cron-runs"] }),
  });

  const projectList = projects.data?.projects ?? [];
  const cronJobList = cronJobs.data?.jobs ?? [];
  const filteredCronJobs =
    cronProjectFilter === ""
      ? cronJobList
      : cronProjectFilter === "__workspace__"
        ? cronJobList.filter((j) => !j.project_id)
        : cronJobList.filter((j) => j.project_id === cronProjectFilter);
  const cronRunList = cronRuns.data?.runs ?? [];
  const jobProjectById = useMemo(
    () => new Map(cronJobList.map((j) => [j.id, j.project_id ?? undefined])),
    [cronJobList],
  );
  const pipelineChecks = (doctor.data?.doctor.checks ?? []).filter((c) =>
    PIPELINE_DOCTOR_IDS.includes(c.id as (typeof PIPELINE_DOCTOR_IDS)[number]),
  );
  const automationNotifyPolicies = (notificationPolicies.data?.policies ?? []).filter((p) =>
    AUTOMATION_NOTIFY_EVENTS.has(p.event_type),
  );

  return (
    <div className={`flex flex-col gap-4 ${className}`}>
      <SectionCard
        title={t("automations.cronRuns")}
        noPadding
        action={
          <button
            type="button"
            className="dw-btn-secondary text-xs inline-flex items-center gap-1"
            onClick={() => {
              void queryClient.invalidateQueries({ queryKey: ["cron-runs"] });
            }}
          >
            <Icon name="refresh" size={14} />
            {t("automations.refresh")}
          </button>
        }
      >
        <CronRunsTable
          runs={cronRunList}
          loading={cronRuns.isLoading}
          retrying={retryCron.isPending}
          onRetry={(jobId) =>
            retryCron.mutate({
              job_id: jobId,
              project_id: jobProjectById.get(jobId),
            })
          }
        />
      </SectionCard>

      <SectionCard title={t("automations.notificationPipeline")}>
        <PipelineSummary checks={pipelineChecks} policies={automationNotifyPolicies} />
      </SectionCard>

      <SectionCard title={t("automations.cronJobs")} noPadding>
        <div className="px-4 py-3 border-b border-outline-variant">
          <select
            className="dw-input text-xs"
            value={cronProjectFilter}
            onChange={(e) => setCronProjectFilter(e.target.value)}
          >
            <option value="">{t("automations.allProjects")}</option>
            <option value="__workspace__">{t("automations.wholeWorkspace")}</option>
            {projectList.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
        <p className="text-xs text-secondary m-0 px-4 py-3">
          {t("automations.registerHint")}{" "}
          {cronJobs.data?.orchestration_path && (
            <code className="font-code break-all">{cronJobs.data.orchestration_path}</code>
          )}
        </p>
        <DataTable
          isEmpty={filteredCronJobs.length === 0}
          empty={<DataTableEmpty message={t("automations.noCronJobs")} />}
        >
          <thead>
            <tr>
              <th>{t("common.id")}</th>
              <th>{t("automations.schedule")}</th>
              <th>{t("common.status")}</th>
            </tr>
          </thead>
          <tbody>
            {filteredCronJobs.map((j) => (
              <tr key={j.id}>
                <td>
                  <code className="font-code text-xs">{j.id.slice(0, 8)}…</code>
                </td>
                <td className="text-secondary text-xs font-code">{j.schedule}</td>
                <td>
                  <StatusBadge status={j.enabled ?? true ? "ok" : "disabled"} />
                </td>
              </tr>
            ))}
          </tbody>
        </DataTable>
      </SectionCard>

      <OrchestrationTasksPanel />
    </div>
  );
}

function PipelineSummary({
  checks,
  policies,
}: {
  checks: DoctorCheck[];
  policies: NotificationPolicyRecord[];
}) {
  const t = useT();
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <ul className="list-none m-0 p-0 flex flex-col gap-2">
        {checks.map((c) => (
          <li
            key={c.id}
            className="flex items-start gap-2 text-sm border border-outline-variant rounded-lg px-3 py-2"
          >
            <StatusBadge status={c.status === "ok" ? "ok" : c.status === "error" ? "error" : "warn"} />
            <span className="text-xs text-secondary break-words">{c.message}</span>
          </li>
        ))}
      </ul>
      <div>
        {policies.length === 0 ? (
          <p className="text-sm text-secondary m-0">{t("automations.noNotificationPolicies")}</p>
        ) : (
          <ul className="list-none m-0 p-0 flex flex-col gap-2">
            {policies.map((p) => (
              <li key={p.id} className="text-sm border border-outline-variant rounded-lg px-3 py-2">
                <code className="font-code text-xs">{p.event_type}</code>
              </li>
            ))}
          </ul>
        )}
        <Link to="/settings" className="inline-block text-xs text-primary mt-3 no-underline hover:underline">
          {t("automations.openNotificationSettings")}
        </Link>
      </div>
    </div>
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
    <DataTable isEmpty={runs.length === 0} empty={<DataTableEmpty message={t("automations.noCronRuns")} />}>
      <thead>
        <tr>
          <th>{t("automations.job")}</th>
          <th>{t("common.status")}</th>
          <th>{t("automations.time")}</th>
          <th>{t("common.actions")}</th>
        </tr>
      </thead>
      <tbody>
        {runs.map((r) => (
          <tr key={`${r.line_no}-${r.fired_at}`}>
            <td>
              <code className="font-code text-xs">{r.job_id.slice(0, 8)}…</code>
            </td>
            <td>
              <StatusBadge
                status={
                  r.status === "ok"
                    ? "ok"
                    : r.status === "error" || r.status === "failed"
                      ? "error"
                      : r.status
                }
              />
            </td>
            <td className="text-secondary text-xs">{r.fired_at}</td>
            <td>
              {(r.status === "failed" || r.status === "error") && (
                <button
                  type="button"
                  className="dw-btn-secondary text-xs"
                  disabled={retrying}
                  onClick={() => onRetry(r.job_id)}
                >
                  {t("automations.retryRun")}
                </button>
              )}
              {r.dashboard_session_id && (
                <Link
                  to="/conversations"
                  search={sessionChatSearch(r.dashboard_session_id)}
                  className="text-xs ml-2"
                >
                  {t("automations.view")}
                </Link>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </DataTable>
  );
}
