import { Icon } from "@/components/Icon";
import type { BillingCycle, PlanCatalogEntry, PlanTier } from "@/api/types/service";
import { useT } from "@/i18n/context";

export function PlanTierCard({
  catalog,
  current,
  highlighted,
  billingCycle,
  onSelect,
}: {
  catalog: PlanCatalogEntry;
  current: PlanTier;
  highlighted?: boolean;
  billingCycle: BillingCycle;
  onSelect: (tier: PlanTier) => void;
}) {
  const t = useT();
  const isCurrent = catalog.tier === current;
  const priceUsd =
    billingCycle === "yearly" ? catalog.yearlyPriceUsd : catalog.monthlyPriceUsd;
  const price =
    priceUsd === 0
      ? t("service.plan.freePrice")
      : billingCycle === "yearly"
        ? t("service.plan.pricePerYear").replace("{price}", String(priceUsd))
        : t("service.plan.pricePerMonth").replace("{price}", String(priceUsd));

  return (
    <div
      className={`console-plan-card glass-card flex flex-col h-full ${
        highlighted ? "console-plan-card--featured" : ""
      } ${isCurrent ? "console-plan-card--current" : ""}`}
    >
      <div className="p-4 flex flex-col flex-1 gap-4">
        <div>
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <h3 className="text-lg font-semibold m-0">{t(`service.plan.tiers.${catalog.tier}`)}</h3>
            {highlighted && (
              <span className="console-plan-badge console-plan-badge--featured">
                {t("service.plan.recommended")}
              </span>
            )}
            {isCurrent && (
              <span className="console-plan-badge console-plan-badge--current">
                {t("service.plan.current")}
              </span>
            )}
          </div>
          <p className="text-sm text-secondary mt-1 mb-0">
            {t(`service.plan.tierDesc.${catalog.tier}`)}
          </p>
          <p className="text-2xl font-bold mt-3 mb-0 tabular-nums">{price}</p>
        </div>

        <ul className="space-y-2 text-sm m-0 p-0 list-none flex-1">
          {catalog.featureKeys.map((key) => (
            <li key={key} className="flex items-start gap-2 text-secondary">
              <Icon name="check_circle" size={16} className="text-success shrink-0 mt-0.5" />
              <span>{t(key)}</span>
            </li>
          ))}
        </ul>

        <button
          type="button"
          className={
            isCurrent
              ? "dw-btn-secondary w-full"
              : highlighted
                ? "dw-btn-primary w-full"
                : "dw-btn-secondary w-full"
          }
          disabled={isCurrent}
          onClick={() => onSelect(catalog.tier)}
        >
          {isCurrent
            ? t("service.plan.currentPlan")
            : catalog.tier === "team"
              ? t("service.plan.contactTeam")
              : t("service.plan.upgradeTo").replace(
                  "{tier}",
                  t(`service.plan.tiers.${catalog.tier}`),
                )}
        </button>
      </div>
    </div>
  );
}
