import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { AnalyticsBlock } from "@/components/KpiMetricGrid";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";
import { sessionChatSearch } from "@/lib/sessionLinks";

/** Security activity log: denied tools and logged approval-pending events from output.log. */
export function SecurityActivityPanel({
  projectId,
  variant = "card",
}: {
  projectId?: string;
  variant?: "card" | "analytics";
}) {
  const t = useT();
  const activity = useQuery({
    queryKey: ["security-activity", projectId ?? ""],
    queryFn: () => api.securityActivity({ limit: 8, projectId }),
    refetchInterval: 15_000,
  });

  const summary = activity.data?.summary;
  const rows = summary?.recent ?? [];
  const isAnalytics = variant === "analytics";
  const title = t("home.securityActivity");

  const wrap = (body: ReactNode) =>
    isAnalytics ? (
      <AnalyticsBlock title={title}>{body}</AnalyticsBlock>
    ) : (
      <SectionCard title={title}>{body}</SectionCard>
    );

  if (!summary && activity.isLoading) {
    return wrap(<p className="text-sm text-secondary m-0">{t("common.loading")}</p>);
  }

  if (!summary || (rows.length === 0 && summary.denied_total === 0 && summary.pending_total === 0)) {
    return wrap(
      <div className={isAnalytics ? "dw-analytics-empty" : "dw-empty-compact"}>
        <Icon name="verified_user" size={isAnalytics ? 28 : 32} className="text-success/80 mb-2" />
        <p className="text-sm text-secondary m-0">{t("home.securityEmpty")}</p>
      </div>,
    );
  }

  return wrap(
    <>
      <div className="flex flex-wrap gap-2 mb-3">
        <SecurityStat label={t("home.securityDenied")} value={summary.denied_total} tone="error" />
        <SecurityStat label={t("home.securityPending")} value={summary.pending_total} tone="warn" />
      </div>
      <p className="text-xs text-secondary m-0 mb-3">{summary.note || t("home.securityActivityNote")}</p>
      <div className="overflow-x-auto rounded-lg border border-outline-variant/40">
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
                  <code className="font-code text-xs">{e.tool_name || e.title}</code>
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
                </td>
                <td className="text-secondary text-xs max-w-[200px] truncate">
                  {e.reason ?? "—"}
                </td>
                <td className="text-xs">
                  {e.session_id ? (
                    <Link
                      to="/conversations"
                      search={sessionChatSearch(e.session_id, e.project_id ?? undefined)}
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
    </>,
  );
}

function SecurityStat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: "error" | "warn";
}) {
  return (
    <div
      className={`inline-flex items-center gap-2 rounded-lg px-3 py-2 border ${
        tone === "error"
          ? "bg-error-container/20 border-error/20"
          : "bg-warn-container/20 border-warn/25"
      }`}
    >
      <span className="text-xs text-secondary">{label}</span>
      <span className="text-base font-semibold tabular-nums text-on-surface">{value}</span>
    </div>
  );
}
