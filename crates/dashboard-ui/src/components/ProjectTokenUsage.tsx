import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { AnalyticsBlock, KpiMetricGrid } from "@/components/KpiMetricGrid";
import { ModelUsageTable } from "@/components/ModelUsageTable";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function ProjectTokenUsage({ projectId }: { projectId: string }) {
  const t = useT();
  const [showModels, setShowModels] = useState(false);
  const usage = useQuery({
    queryKey: ["project-usage", projectId, 7],
    queryFn: () => api.projectUsage(projectId, 7),
    staleTime: 120_000,
  });

  const u = usage.data?.usage;
  const modelRows = usage.data?.by_model ?? [];
  if (!u) return null;

  const hasUsage =
    u.llm_calls > 0 || u.total_tokens > 0 || u.estimated_cost_usd > 0 || modelRows.length > 0;
  if (!hasUsage) return null;

  return (
    <AnalyticsBlock
      title={t("projectDetail.tokenUsage")}
      action={
        <a
          href={api.usageExportUrl(u.days, projectId)}
          className="dw-btn-ghost text-xs no-underline shrink-0"
          download={`token-usage-${projectId}.csv`}
        >
          <Icon name="download" size={16} />
          {t("home.tokenExport")}
        </a>
      }
      footer={
        <p className="text-xs text-secondary m-0">
          {t("home.tokenWindow").replace("{days}", String(u.days))}
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
      {modelRows.length > 0 && (
        <div className="mt-3">
          <button
            type="button"
            className="dw-btn-ghost text-xs"
            onClick={() => setShowModels((v) => !v)}
          >
            <Icon name={showModels ? "expand_less" : "expand_more"} size={16} />
            {t("projectDetail.modelBreakdownToggle").replace("{n}", String(modelRows.length))}
          </button>
          {showModels && <ModelUsageTable rows={modelRows} />}
        </div>
      )}
    </AnalyticsBlock>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}
