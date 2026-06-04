import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { AnalyticsBlock, KpiMetricGrid } from "@/components/KpiMetricGrid";
import { ModelUsageTable } from "@/components/ModelUsageTable";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function HomeTokenUsage() {
  const t = useT();
  const usage = useQuery({
    queryKey: ["usage-metrics", 7],
    queryFn: () => api.usageMetrics(7),
    staleTime: 120_000,
  });

  const u = usage.data?.usage;
  if (!u) return null;

  return (
    <AnalyticsBlock
      title={t("home.tokenUsage")}
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
          {t("home.tokenCostHint")}
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
      <ModelUsageTable rows={usage.data?.by_model ?? []} />
    </AnalyticsBlock>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}
