import { Link } from "@tanstack/react-router";
import { QuotaProgressBar } from "@/components/service/QuotaProgressBar";
import type { ServiceSection } from "@/components/service/ServiceNav";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { quotaPercent } from "@/lib/planCatalog";
import { useT } from "@/i18n/context";

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function KpiCard({
  label,
  value,
  hint,
  to,
  search,
}: {
  label: string;
  value: string;
  hint: string;
  to: string;
  search: { section: ServiceSection };
}) {
  return (
    <Link to={to} search={search} className="console-kpi-card glass-card no-underline">
      <small className="text-secondary text-xs block mb-2">{label}</small>
      <strong className="text-xl font-bold text-on-surface block tracking-tight">{value}</strong>
      <span className="text-xs text-secondary mt-1 block">{hint}</span>
    </Link>
  );
}

export function ServiceOverviewSection() {
  const t = useT();
  const { entitlements } = useAccountCloud();
  if (!entitlements) return null;

  const { plan, subscriptionStatus, billingCycle, quota, billingPeriod } = entitlements;
  const statusOk = subscriptionStatus === "active" || subscriptionStatus === "trialing";

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold m-0 tracking-tight">{t("service.overview.title")}</h2>
          <p className="text-sm text-secondary m-0 mt-1">
            {t("service.overview.subtitle")
              .replace("{plan}", t(`service.plan.tiers.${plan}`))
              .replace("{start}", billingPeriod.start)
              .replace("{end}", billingPeriod.end)}
          </p>
        </div>
        <StatusBadge
          status={statusOk ? "ok" : "warn"}
          label={t(`service.status.${subscriptionStatus}`)}
        />
      </div>

      <div className="console-kpi-grid">
        <KpiCard
          label={t("service.overview.kpi.plan")}
          value={t(`service.plan.tiers.${plan}`)}
          hint={t(`service.billing.cycle.${billingCycle}`)}
          to="/account"
          search={{ section: "plan" }}
        />
        <KpiCard
          label={t("service.overview.kpi.tokens")}
          value={`${formatTokens(quota.tokenUsed)} / ${formatTokens(quota.tokenLimit)}`}
          hint={t("service.usage.periodRemaining").replace(
            "{days}",
            String(billingPeriod.daysRemaining),
          )}
          to="/account"
          search={{ section: "usage" }}
        />
        <KpiCard
          label={t("service.overview.kpi.apiKeys")}
          value={`${quota.apiKeyUsed} / ${quota.apiKeyLimit}`}
          hint={t("service.overview.kpi.apiKeysHint")}
          to="/account"
          search={{ section: "api" }}
        />
        <KpiCard
          label={t("service.overview.kpi.seats")}
          value={`${quota.seatUsed} / ${quota.seatLimit}`}
          hint={t("service.overview.kpi.seatsHint")}
          to="/account"
          search={{ section: "enterprise" }}
        />
      </div>

      <SectionCard title={t("service.overview.quotaTitle")}>
        <QuotaProgressBar
          label={t("service.usage.tokenQuota")}
          used={quota.tokenUsed}
          limit={quota.tokenLimit}
          unit={t("service.usage.tokens")}
        />
        <p className="text-xs text-secondary m-0 mt-3">
          {t("service.overview.quotaPct").replace(
            "{pct}",
            String(quotaPercent(quota.tokenUsed, quota.tokenLimit)),
          )}
        </p>
      </SectionCard>

      <div className="flex flex-wrap gap-2">
        <Link to="/account" search={{ section: "plan" }} className="dw-btn-primary no-underline text-sm">
          {t("service.overview.cta.upgrade")}
        </Link>
        <Link to="/account" search={{ section: "billing" }} className="dw-btn-secondary no-underline text-sm">
          {t("service.overview.cta.billing")}
        </Link>
        <Link to="/account" search={{ section: "api" }} className="dw-btn-ghost no-underline text-sm">
          {t("service.overview.cta.apiKeys")}
        </Link>
      </div>
    </div>
  );
}
