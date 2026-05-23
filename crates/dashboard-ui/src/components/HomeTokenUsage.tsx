import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ModelUsageTable } from "@/components/ModelUsageTable";
import { SectionCard } from "@/components/ui/SectionCard";
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
    <SectionCard title={t("home.tokenUsage")}>
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-3">
        <Mini label={t("home.tokenCalls")} value={String(u.llm_calls)} />
        <Mini label={t("home.tokenInput")} value={formatTokens(u.input_tokens)} />
        <Mini label={t("home.tokenOutput")} value={formatTokens(u.output_tokens)} />
        <Mini label={t("home.tokenTotal")} value={formatTokens(u.total_tokens)} highlight />
        <Mini
          label={t("home.tokenCost")}
          value={`$${u.estimated_cost_usd.toFixed(2)}`}
          highlight
        />
      </div>
      <ModelUsageTable rows={usage.data?.by_model ?? []} />
      <p className="text-xs text-secondary m-0 mt-2">{t("home.tokenWindow").replace("{days}", String(u.days))}</p>
      <p className="text-[10px] text-outline m-0 mt-1">{t("home.tokenCostHint")}</p>
      <a
        href={api.usageExportUrl(u.days)}
        className="dw-btn-secondary inline-block mt-3 no-underline text-sm"
        download="token-usage.csv"
      >
        {t("home.tokenExport")}
      </a>
    </SectionCard>
  );
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function Mini({ label, value, highlight }: { label: string; value: string; highlight?: boolean }) {
  return (
    <div className="dw-stat-card">
      <div className="dw-stat-label">{label}</div>
      <div className={`dw-stat-value text-sm ${highlight ? "text-primary" : ""}`}>{value}</div>
    </div>
  );
}
