import { Link } from "@tanstack/react-router";
import type { ServiceEntitlements } from "@/api/types/service";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { QuotaProgressBar } from "@/components/service/QuotaProgressBar";
import { useT } from "@/i18n/context";

export function CurrentPlanSummary({ entitlements }: { entitlements: ServiceEntitlements }) {
  const t = useT();
  const { plan, subscriptionStatus, billingCycle, quota, billingPeriod } = entitlements;

  return (
    <SectionCard title={t("service.plan.currentSubscription")}>
      <div className="flex flex-wrap items-center gap-2 mb-4">
        <span className="text-xl font-bold">{t(`service.plan.tiers.${plan}`)}</span>
        <StatusBadge
          status={subscriptionStatus === "active" || subscriptionStatus === "trialing" ? "ok" : "warn"}
          label={t(`service.status.${subscriptionStatus}`)}
        />
        <span className="text-xs text-secondary px-2 py-0.5 rounded-full bg-surface-container-high">
          {t(`service.billing.cycle.${billingCycle}`)}
        </span>
      </div>
      <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0 mb-4">
        <dt className="text-secondary m-0">{t("service.billing.period")}</dt>
        <dd className="m-0 tabular-nums">
          {billingPeriod.start} — {billingPeriod.end}
        </dd>
        <dt className="text-secondary m-0">{t("service.billing.daysRemaining")}</dt>
        <dd className="m-0">{billingPeriod.daysRemaining}</dd>
        <dt className="text-secondary m-0">{t("service.plan.seats")}</dt>
        <dd className="m-0">
          {quota.seatUsed} / {quota.seatLimit}
        </dd>
      </dl>
      <QuotaProgressBar
        label={t("service.usage.tokenQuota")}
        used={quota.tokenUsed}
        limit={quota.tokenLimit}
        unit={t("service.usage.tokens")}
      />
      <Link to="/account" search={{ section: "billing" }} className="inline-block mt-4 text-sm">
        {t("service.plan.viewBilling")}
      </Link>
    </SectionCard>
  );
}

export function UpgradeValueCard() {
  const t = useT();
  const bullets = [
    "service.plan.upgradeValue.quota",
    "service.plan.upgradeValue.automation",
    "service.plan.upgradeValue.api",
    "service.plan.upgradeValue.team",
  ] as const;

  return (
    <SectionCard title={t("service.plan.whyUpgrade")}>
      <ul className="space-y-2 text-sm m-0 p-0 list-none">
        {bullets.map((key) => (
          <li key={key} className="text-secondary">
            {t(key)}
          </li>
        ))}
      </ul>
    </SectionCard>
  );
}
