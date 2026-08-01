import { Link } from "@tanstack/react-router";
import type { ServiceEntitlements } from "@/api/types/service";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { QuotaProgressBar } from "@/components/service/QuotaProgressBar";
import { useT } from "@/i18n/context";

/** Compact horizontal banner — half width on desktop to leave room for plan cards below. */
export function CurrentPlanSummary({ entitlements }: { entitlements: ServiceEntitlements }) {
  const t = useT();
  const { plan, subscriptionStatus, billingCycle, quota, billingPeriod } = entitlements;

  return (
    <div className="console-plan-banner glass-card">
      <div className="console-plan-banner__head">
        <span className="console-plan-banner__kicker">{t("service.plan.currentSubscription")}</span>
        <div className="console-plan-banner__plan-row">
          <span className="console-plan-banner__plan">{t(`service.plan.tiers.${plan}`)}</span>
          <StatusBadge
            status={
              subscriptionStatus === "active" || subscriptionStatus === "trialing" ? "ok" : "warn"
            }
            label={t(`service.status.${subscriptionStatus}`)}
          />
          <span className="console-plan-banner__cycle">{t(`service.billing.cycle.${billingCycle}`)}</span>
        </div>
      </div>

      <div className="console-plan-banner__meta">
        <span>
          {billingPeriod.start} — {billingPeriod.end}
        </span>
        <span className="console-plan-banner__dot" aria-hidden>
          ·
        </span>
        <span>
          {t("service.billing.daysRemaining")} {billingPeriod.daysRemaining}
        </span>
        <span className="console-plan-banner__dot" aria-hidden>
          ·
        </span>
        <span>
          {t("service.plan.seats")} {quota.seatUsed}/{quota.seatLimit}
        </span>
      </div>

      <div className="console-plan-banner__quota">
        <QuotaProgressBar
          label={t("service.usage.tokenQuota")}
          used={quota.tokenUsed}
          limit={quota.tokenLimit}
          unit={t("service.usage.tokens")}
        />
      </div>

      <Link
        to="/account"
        search={{ section: "billing" }}
        className="console-plan-banner__link text-sm shrink-0"
      >
        {t("service.plan.viewBilling")}
      </Link>
    </div>
  );
}
