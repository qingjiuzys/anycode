import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
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
    <SectionCard title={t("home.savedHours")}>
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-3">
        <Mini label={t("home.savedHoursSessions")} value={String(k.sessions_completed)} />
        <Mini label={t("home.savedHoursAutomation")} value={formatHours(k.automation_hours)} />
        <Mini label={t("home.savedHoursManual")} value={formatHours(k.estimated_manual_hours)} />
        <Mini label={t("home.savedHoursSaved")} value={formatHours(k.estimated_saved_hours)} highlight />
        <Mini
          label={t("home.savedHoursValue")}
          value={`$${k.estimated_value_usd.toFixed(0)}`}
          highlight
        />
      </div>
      <p className="text-xs text-secondary m-0 mt-2">
        {t("home.tokenWindow").replace("{days}", String(k.days))}
      </p>
      <p className="text-[10px] text-outline m-0 mt-1">{t("home.savedHoursHint")}</p>
    </SectionCard>
  );
}

function formatHours(h: number): string {
  if (h >= 100) return `${h.toFixed(0)}h`;
  if (h >= 10) return `${h.toFixed(1)}h`;
  return `${h.toFixed(2)}h`;
}

function Mini({ label, value, highlight }: { label: string; value: string; highlight?: boolean }) {
  return (
    <div className="dw-stat-card">
      <div className="dw-stat-label">{label}</div>
      <div className={`dw-stat-value text-sm ${highlight ? "text-primary" : ""}`}>{value}</div>
    </div>
  );
}
