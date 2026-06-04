import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { AnalyticsBlock, KpiMetricGrid } from "@/components/KpiMetricGrid";
import { useT } from "@/i18n/context";

export function HomeSavedHoursKpi() {
  const t = useT();
  const kpi = useQuery({
    queryKey: ["saved-hours-kpi", 7],
    queryFn: () => api.savedHoursKpi(7),
    staleTime: 120_000,
  });

  const k = kpi.data?.kpi;
  if (!k) return null;

  return (
    <AnalyticsBlock
      title={t("home.savedHours")}
      footer={
        <p className="text-xs text-secondary m-0 leading-relaxed">
          {t("home.tokenWindow").replace("{days}", String(k.days))}
          <span className="text-on-surface-variant/60 mx-1.5">·</span>
          {t("home.savedHoursHint")}
        </p>
      }
    >
      <KpiMetricGrid
        metrics={[
          { label: t("home.savedHoursSessions"), value: String(k.sessions_completed) },
          { label: t("home.savedHoursAutomation"), value: formatHours(k.automation_hours) },
          { label: t("home.savedHoursManual"), value: formatHours(k.estimated_manual_hours) },
          {
            label: t("home.savedHoursSaved"),
            value: formatHours(k.estimated_saved_hours),
            highlight: true,
          },
          {
            label: t("home.savedHoursValue"),
            value: `$${k.estimated_value_usd.toFixed(0)}`,
            highlight: true,
          },
        ]}
      />
    </AnalyticsBlock>
  );
}

function formatHours(h: number): string {
  if (h >= 100) return `${h.toFixed(0)}h`;
  if (h >= 10) return `${h.toFixed(1)}h`;
  return `${h.toFixed(2)}h`;
}
