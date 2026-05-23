import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ModelUsageTable } from "@/components/ModelUsageTable";
import { SessionTokenChart } from "@/components/SessionTokenChart";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SessionTokenUsage({ sessionId }: { sessionId: string }) {
  const t = useT();
  const usage = useQuery({
    queryKey: ["session-usage", sessionId],
    queryFn: () => api.sessionUsage(sessionId),
    staleTime: 120_000,
  });

  const u = usage.data?.usage;
  if (!u || u.total_tokens === 0) return null;

  return (
    <SectionCard title={t("session.tokenUsage")}>
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <Mini label={t("home.tokenCalls")} value={String(u.llm_calls)} />
        <Mini label={t("home.tokenTotal")} value={formatTokens(u.total_tokens)} highlight />
        <Mini
          label={t("home.tokenCost")}
          value={`$${u.estimated_cost_usd.toFixed(2)}`}
          highlight
        />
      </div>
      <SessionTokenChart rows={usage.data?.by_model ?? []} />
      <ModelUsageTable rows={usage.data?.by_model ?? []} />
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
