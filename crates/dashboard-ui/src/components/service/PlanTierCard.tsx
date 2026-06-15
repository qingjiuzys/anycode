import { Icon } from "@/components/Icon";
import type { PlanCatalogEntry, PlanTier } from "@/api/types/service";
import { useT } from "@/i18n/context";

export function PlanTierCard({
  catalog,
  current,
  highlighted,
  onSelect,
}: {
  catalog: PlanCatalogEntry;
  current: PlanTier;
  highlighted?: boolean;
  onSelect: (tier: PlanTier) => void;
}) {
  const t = useT();
  const isCurrent = catalog.tier === current;
  const price =
    catalog.monthlyPriceUsd === 0
      ? t("service.plan.freePrice")
      : t("service.plan.pricePerMonth").replace("{price}", String(catalog.monthlyPriceUsd));

  return (
    <div
      className={`dw-section-card flex flex-col h-full ${
        highlighted ? "ring-2 ring-primary/40 shadow-md" : ""
      } ${isCurrent ? "border-primary/30" : ""}`}
    >
      <div className="p-4 flex flex-col flex-1 gap-4">
        <div>
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-lg font-semibold m-0">{t(`service.plan.tiers.${catalog.tier}`)}</h3>
            {highlighted && (
              <span className="text-[10px] uppercase tracking-wide font-semibold px-2 py-0.5 rounded-full bg-primary/15 text-primary">
                {t("service.plan.recommended")}
              </span>
            )}
            {isCurrent && (
              <span className="text-[10px] uppercase tracking-wide font-semibold px-2 py-0.5 rounded-full bg-success/15 text-success">
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
          className={isCurrent ? "dw-btn-secondary w-full" : highlighted ? "dw-btn-primary w-full" : "dw-btn-secondary w-full"}
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
