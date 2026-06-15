import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { HomeSavedHoursKpi } from "@/components/HomeSavedHoursKpi";
import { HomeTimelineChart } from "@/components/HomeTimelineChart";
import { ModelUsageTable } from "@/components/ModelUsageTable";
import { AnalyticsBlock, KpiMetricGrid } from "@/components/KpiMetricGrid";
import { QuotaProgressBar } from "@/components/service/QuotaProgressBar";
import { UpgradePromptCard } from "@/components/service/UpgradePromptCard";
import { SectionCard } from "@/components/ui/SectionCard";
import { Icon } from "@/components/Icon";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { isQuotaNearLimit } from "@/lib/planCatalog";
import { useT } from "@/i18n/context";

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

export function ServiceUsageSection() {
  const t = useT();
  const { entitlements, usageStats, usageByModel, usageLoading } = useAccountCloud();
  const timeline = useQuery({
    queryKey: ["timeline-metrics", 30],
    queryFn: () => api.timelineMetrics(30),
    staleTime: 120_000,
  });

  if (!entitlements) return null;

  const u = usageStats;
  const showUpgrade = isQuotaNearLimit(
    entitlements.quota.tokenUsed,
    entitlements.quota.tokenLimit,
  );

  return (
    <div className="space-y-6">
      <SectionCard title={t("service.usage.quotaOverview")}>
        <div className="space-y-4">
          <QuotaProgressBar
            label={t("service.usage.tokenQuota")}
            used={entitlements.quota.tokenUsed}
            limit={entitlements.quota.tokenLimit}
            unit={t("service.usage.tokens")}
          />
          <p className="text-xs text-secondary m-0">
            {t("service.usage.periodRemaining").replace(
              "{days}",
              String(entitlements.billingPeriod.daysRemaining),
            )}
          </p>
        </div>
      </SectionCard>

      {showUpgrade && <UpgradePromptCard />}

      {usageLoading || !u ? (
        <p className="text-sm text-secondary">{t("common.loading")}</p>
      ) : (
        <AnalyticsBlock
          title={t("service.usage.metricsTitle")}
          action={
            <a
              href={api.usageExportUrl(u.days)}
              className="dw-btn-ghost text-xs no-underline shrink-0"
              download="token-usage.csv"
            >
              <Icon name="download" size={16} />
              {t("home.tokenExport")}
            </a>
          }
          footer={
            <p className="text-xs text-secondary m-0 leading-relaxed">
              {t("home.tokenWindow").replace("{days}", String(u.days))}
              <span className="text-on-surface-variant/60 mx-1.5">·</span>
              {t("service.usage.byokHint")}
            </p>
          }
        >
          <KpiMetricGrid
            metrics={[
              { label: t("home.tokenCalls"), value: String(u.llm_calls) },
              { label: t("home.tokenInput"), value: formatTokens(u.input_tokens) },
              { label: t("home.tokenOutput"), value: formatTokens(u.output_tokens) },
              { label: t("home.tokenTotal"), value: formatTokens(u.total_tokens), highlight: true },
              {
                label: t("home.tokenCost"),
                value: `$${u.estimated_cost_usd.toFixed(2)}`,
                highlight: true,
              },
            ]}
          />
          <ModelUsageTable rows={usageByModel} />
        </AnalyticsBlock>
      )}

      <HomeSavedHoursKpi />

      <SectionCard title={t("home.timeline7d")} noPadding className="dw-analytics-chart-card">
        <HomeTimelineChart timeline={timeline.data?.timeline} tall />
      </SectionCard>
    </div>
  );
}
