import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { AnalyticsBlock, KpiMetricGrid } from "@/components/KpiMetricGrid";
import { ModelUsageTable } from "@/components/ModelUsageTable";
import { ProjectTokenChart } from "@/components/ProjectTokenChart";
import { SessionTokenChart } from "@/components/SessionTokenChart";
import { TokenTimelineChart } from "@/components/TokenTimelineChart";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const DAY_OPTIONS = [7, 30, 90] as const;

export function HomeTokenUsage() {
  const t = useT();
  const [days, setDays] = useState<(typeof DAY_OPTIONS)[number]>(7);
  const usage = useQuery({
    queryKey: ["usage-metrics", days],
    queryFn: () => api.usageMetrics(days),
    staleTime: 120_000,
  });

  if (usage.isLoading) {
    return (
      <AnalyticsBlock title={t("home.tokenUsage")}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </AnalyticsBlock>
    );
  }

  if (usage.isError) {
    return (
      <AnalyticsBlock title={t("home.tokenUsage")}>
        <p className="text-sm text-error m-0">{t("home.apiError")}</p>
      </AnalyticsBlock>
    );
  }

  const u = usage.data?.usage;
  if (!u) return null;

  const byModel = usage.data?.by_model ?? [];
  const byProject = usage.data?.by_project ?? [];
  const byDay = usage.data?.by_day ?? [];
  const isEmpty = u.llm_calls === 0 && u.total_tokens === 0;

  return (
    <AnalyticsBlock
      title={t("home.tokenUsage")}
      action={
        <div className="flex items-center gap-2 shrink-0">
          <select
            className="dw-input text-xs py-1 px-2 min-w-0"
            value={days}
            onChange={(e) => setDays(Number(e.target.value) as (typeof DAY_OPTIONS)[number])}
            aria-label={t("home.tokenDays")}
          >
            {DAY_OPTIONS.map((d) => (
              <option key={d} value={d}>
                {t("home.tokenDays").replace("{days}", String(d))}
              </option>
            ))}
          </select>
          <a
            href={api.usageExportUrl(u.days)}
            className="dw-btn-ghost text-xs no-underline shrink-0"
            download="token-usage.csv"
          >
            <Icon name="download" size={16} />
            {t("home.tokenExport")}
          </a>
        </div>
      }
      footer={
        <p className="text-xs text-secondary m-0 leading-relaxed">
          {t("home.tokenWindow").replace("{days}", String(u.days))}
          <span className="text-on-surface-variant/60 mx-1.5">·</span>
          {t("home.tokenCostHint")}
        </p>
      }
    >
      {isEmpty ? (
        <p className="text-sm text-secondary m-0 dw-analytics-empty">{t("charts.noTokenUsage")}</p>
      ) : (
        <>
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
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
            <SectionCard title={t("charts.tokenTimeline")} noPadding className="dw-analytics-chart-card">
              <TokenTimelineChart points={byDay} />
            </SectionCard>
            <SectionCard title={t("charts.byProject")} noPadding className="dw-analytics-chart-card">
              <div className="px-2 pb-2">
                <ProjectTokenChart rows={byProject} />
              </div>
            </SectionCard>
          </div>
          <SectionCard title={t("home.tokenChart")} noPadding className="dw-analytics-chart-card mt-4">
            <div className="px-2 pb-2">
              <SessionTokenChart rows={byModel} />
            </div>
          </SectionCard>
          <ModelUsageTable rows={byModel} />
        </>
      )}
    </AnalyticsBlock>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}
