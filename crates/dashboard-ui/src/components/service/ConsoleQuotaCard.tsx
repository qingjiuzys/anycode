import { QuotaProgressBar } from "@/components/service/QuotaProgressBar";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";
import type { ServiceSection } from "@/components/service/ServiceNav";

export function ConsoleQuotaCard({ onNavigate }: { onNavigate: (s: ServiceSection) => void }) {
  const t = useT();
  const { entitlements } = useAccountCloud();
  if (!entitlements) return null;

  const { plan, billingCycle, quota, billingPeriod } = entitlements;

  return (
    <button
      type="button"
      className="console-quota-card w-full text-left"
      onClick={() => onNavigate("usage")}
      aria-label={t("service.console.quotaCardAria")}
    >
      <div className="flex items-center justify-between gap-2 mb-2">
        <span className="text-sm font-semibold text-on-surface">
          {t(`service.plan.tiers.${plan}`)}
        </span>
        <span className="text-[11px] text-secondary px-2 py-0.5 rounded-full console-quota-cycle">
          {t(`service.billing.cycle.${billingCycle}`)}
        </span>
      </div>
      <QuotaProgressBar
        label={t("service.usage.tokenQuota")}
        used={quota.tokenUsed}
        limit={quota.tokenLimit}
        unit={t("service.usage.tokens")}
      />
      <p className="text-xs text-secondary m-0 mt-2">
        {t("service.usage.periodRemaining").replace("{days}", String(billingPeriod.daysRemaining))}
      </p>
    </button>
  );
}
