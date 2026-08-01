import type { ReactNode } from "react";
import { Icon } from "@/components/Icon";
import type { BillingCycle, PlanCatalogEntry, PlanTier } from "@/api/types/service";
import { useT } from "@/i18n/context";
import { formatFen } from "@/lib/money";

export function PlanTierCard({
  catalog,
  current,
  highlighted,
  promoLabel,
  billingCycle,
  onCheckout,
  onContactTeam,
  checkoutLoading,
  checkoutCta,
}: {
  catalog: PlanCatalogEntry;
  current: PlanTier;
  highlighted?: boolean;
  promoLabel?: string | null;
  billingCycle: BillingCycle;
  onCheckout?: (tier: PlanTier) => void;
  onContactTeam?: () => void;
  checkoutLoading?: PlanTier | null;
  checkoutCta?: string;
}) {
  const t = useT();
  const isCurrent = catalog.tier === current;
  const isPaidTier = catalog.tier !== "free" && catalog.tier !== "team";
  const priceFen =
    billingCycle === "yearly" ? catalog.yearlyPriceFen : catalog.monthlyPriceFen;
  const price =
    priceFen === 0
      ? t("service.plan.freePrice")
      : billingCycle === "yearly"
        ? t("service.plan.pricePerYear").replace("{price}", formatFen(priceFen))
        : t("service.plan.pricePerMonth").replace("{price}", formatFen(priceFen));
  const badge = promoLabel?.trim() || (highlighted ? t("service.plan.recommended") : null);
  const loading = checkoutLoading === catalog.tier;

  let action: ReactNode = null;
  if (isCurrent) {
    action = (
      <button type="button" className="console-plan-cta console-plan-cta--muted" disabled>
        {t("service.plan.currentPlan")}
      </button>
    );
  } else if (isPaidTier && onCheckout) {
    action = (
      <button
        type="button"
        className="console-plan-cta"
        disabled={loading}
        onClick={() => onCheckout(catalog.tier)}
      >
        {loading ? t("common.loading") : (checkoutCta ?? t("service.plan.wechatSubscribe"))}
      </button>
    );
  } else if (catalog.tier === "team") {
    action = (
      <button type="button" className="console-plan-cta console-plan-cta--outline" onClick={onContactTeam}>
        {t("service.plan.contactTeam")}
      </button>
    );
  }

  return (
    <div
      className={`console-plan-card glass-card ${
        highlighted ? "console-plan-card--featured" : ""
      } ${isCurrent ? "console-plan-card--current" : ""}`}
    >
      <div className="console-plan-card__body">
        <div className="flex items-center justify-between gap-2 flex-wrap">
          <h3 className="text-lg font-semibold m-0">{t(`service.plan.tiers.${catalog.tier}`)}</h3>
          {badge && <span className="console-plan-badge console-plan-badge--featured">{badge}</span>}
          {isCurrent && (
            <span className="console-plan-badge console-plan-badge--current">
              {t("service.plan.current")}
            </span>
          )}
        </div>
        <p className="text-sm text-secondary mt-1 mb-0">{t(`service.plan.tierDesc.${catalog.tier}`)}</p>
        <p className="text-2xl font-bold mt-3 mb-4 tabular-nums">{price}</p>

        <ul className="space-y-2 text-sm m-0 p-0 list-none">
          {catalog.featureKeys.map((key) => (
            <li key={key} className="flex items-start gap-2 text-secondary">
              <Icon name="check_circle" size={16} className="text-success shrink-0 mt-0.5" />
              <span>{t(key)}</span>
            </li>
          ))}
        </ul>
      </div>

      {action && <div className="console-plan-card__actions">{action}</div>}
    </div>
  );
}
