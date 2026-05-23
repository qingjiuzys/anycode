import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

/** Security activity log: denied tools and logged approval-pending events from output.log. */
export function SecurityActivityPanel({ projectId }: { projectId?: string }) {
  const t = useT();
  const activity = useQuery({
    queryKey: ["security-activity", projectId ?? ""],
    queryFn: () => api.securityActivity({ limit: 8, projectId }),
    refetchInterval: 15_000,
  });

  const summary = activity.data?.summary;
  const rows = summary?.recent ?? [];

  if (!summary && activity.isLoading) {
    return (
      <SectionCard title={t("home.securityActivity")}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </SectionCard>
    );
  }

  if (!summary || (rows.length === 0 && summary.denied_total === 0 && summary.pending_total === 0)) {
    return (
      <SectionCard title={t("home.securityActivity")}>
        <EmptyState
          title={t("home.securityEmpty")}
          description={t("home.securityActivityEmpty")}
          icon="policy"
        />
      </SectionCard>
    );
  }

  return (
    <SectionCard title={t("home.securityActivity")}>
      <div className="flex flex-wrap gap-3 mb-3">
        <Stat label={t("home.securityDenied")} value={summary.denied_total} />
        <Stat label={t("home.securityPending")} value={summary.pending_total} />
      </div>
      <p className="text-xs text-secondary m-0 mb-3">{summary.note || t("home.securityActivityNote")}</p>
      <div className="overflow-x-auto">
        <table className="dw-table">
          <thead>
            <tr>
              <th>{t("common.time")}</th>
              <th>{t("home.securityTool")}</th>
              <th>{t("common.status")}</th>
              <th>{t("home.securityReason")}</th>
              <th>{t("nav.projects")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((e) => (
              <tr key={e.id}>
                <td className="text-secondary text-xs whitespace-nowrap">{e.occurred_at}</td>
                <td>
                  <code className="font-code">{e.tool_name || e.title}</code>
                </td>
                <td>
                  <StatusBadge
                    status={
                      e.event_type === "tool_denied"
                        ? "blocked"
                        : e.event_type === "tool_approval_resolved"
                          ? "ok"
                          : "warn"
                    }
                  />
                  <span className="text-xs text-secondary ml-1">
                    {e.event_type === "tool_denied"
                      ? "denied"
                      : e.event_type === "tool_approval_resolved"
                        ? "resolved"
                        : "pending"}
                  </span>
                </td>
                <td className="text-secondary text-xs max-w-[200px] truncate">
                  {e.reason ?? "—"}
                </td>
                <td className="text-xs">
                  {e.session_id ? (
                    <Link
                      to="/sessions/$sessionId"
                      params={{ sessionId: e.session_id }}
                      className="text-primary hover:underline"
                    >
                      {e.project_name}
                    </Link>
                  ) : (
                    e.project_name
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="dw-stat-card min-w-[120px]">
      <div className="dw-stat-label">{label}</div>
      <div className="dw-stat-value">{value}</div>
    </div>
  );
}
