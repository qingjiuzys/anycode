import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { downloadCsv } from "@/utils/exportCsv";
import { useT } from "@/i18n/context";

const RISKS = ["", "low", "medium", "high", "critical"] as const;

function riskLabel(r: string, t: (k: string) => string) {
  if (!r) return t("audit.allRisks");
  return t(`status.${r}`) || r;
}

function exportAudit(
  rows: {
    created_at: string;
    action: string;
    risk: string;
    project_id?: string | null;
    session_id?: string | null;
    actor: string;
  }[],
  t: (k: string) => string,
) {
  downloadCsv("audit-events.csv", [
    [
      t("audit.time"),
      t("audit.action"),
      t("audit.risk"),
      t("audit.project"),
      t("audit.session"),
      t("audit.actor"),
    ],
    ...rows.map((e) => [
      e.created_at,
      e.action,
      e.risk,
      e.project_id ?? "",
      e.session_id ?? "",
      e.actor,
    ]),
  ]);
}

export function AuditPage() {
  const t = useT();
  const [projectId, setProjectId] = useState("");
  const [action, setAction] = useState("");
  const [risk, setRisk] = useState("");

  const projects = useQuery({ queryKey: ["projects"], queryFn: api.projects });
  const audit = useQuery({
    queryKey: ["audit", projectId, action, risk],
    queryFn: () =>
      api.auditEvents({
        projectId: projectId || undefined,
        action: action || undefined,
        risk: risk || undefined,
        limit: 100,
      }),
  });

  const rows = audit.data?.events ?? [];

  return (
    <>
      <PageHeader
        title={t("audit.title")}
        subtitle={t("audit.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("audit.title") },
        ]}
        actions={
          rows.length > 0 ? (
            <button
              type="button"
              className="dw-btn-secondary"
              onClick={() => exportAudit(rows, t)}
            >
              <Icon name="download" size={16} />
              {t("audit.export")}
            </button>
          ) : undefined
        }
      />

      <div className="flex flex-wrap items-center gap-2 mb-4">
        <select
          className="dw-input"
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
        >
          <option value="">{t("audit.allProjects")}</option>
          {(projects.data?.projects ?? []).map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        <input
          type="search"
          className="dw-input w-48"
          placeholder={t("audit.actionFilter")}
          value={action}
          onChange={(e) => setAction(e.target.value)}
        />
        {RISKS.map((r) => (
          <button
            key={r || "all"}
            type="button"
            className={`dw-chip${risk === r ? " active" : ""}`}
            onClick={() => setRisk(r)}
          >
            {riskLabel(r, t)}
          </button>
        ))}
      </div>

      {audit.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}

      {rows.length === 0 && !audit.isLoading && (
        <EmptyState
          title={t("audit.emptyTitle")}
          description={t("audit.emptyDesc")}
          icon="policy"
        />
      )}

      {rows.length > 0 && (
        <div className="dw-section-card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("audit.time")}</th>
                  <th>{t("audit.action")}</th>
                  <th>{t("audit.risk")}</th>
                  <th>{t("audit.project")}</th>
                  <th>{t("audit.session")}</th>
                  <th>{t("audit.actor")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((e) => (
                  <tr key={e.id}>
                    <td className="text-secondary text-xs">{e.created_at}</td>
                    <td>
                      <code className="font-code">{e.action}</code>
                    </td>
                    <td>
                      <StatusBadge status={e.risk === "low" ? "ok" : e.risk} />
                    </td>
                    <td className="text-secondary font-code text-xs">{e.project_id ?? "—"}</td>
                    <td className="text-secondary font-code text-xs">{e.session_id ?? "—"}</td>
                    <td>{e.actor}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </>
  );
}
