import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { BillingCycle, PlanCatalogEntry, PlanTier } from "@/api/types/service";
import { accountCloud } from "@/api/client/accountCloud";
import { PLAN_CATALOG, catalogFromApi } from "@/lib/planCatalog";
import { isDevMockEnabled } from "@/lib/isDevMockEnabled";
import { CurrentPlanSummary, UpgradeValueCard } from "@/components/service/CurrentPlanSummary";
import { PlanTierCard } from "@/components/service/PlanTierCard";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServicePlanSection() {
  const t = useT();
  const { entitlements, setPlan, baseUrl } = useAccountCloud();
  const [billingCycle, setBillingCycle] = useState<BillingCycle>("monthly");
  const [pendingTier, setPendingTier] = useState<PlanTier | null>(null);
  const [error, setError] = useState<string | null>(null);
  const devMock = isDevMockEnabled();

  const catalogQuery = useQuery({
    queryKey: ["plans-catalog", baseUrl],
    queryFn: () => accountCloud.plansCatalog(baseUrl!),
    enabled: Boolean(baseUrl),
    staleTime: 60_000,
  });

  const catalogEntries: PlanCatalogEntry[] =
    catalogQuery.data?.plans != null
      ? catalogFromApi(catalogQuery.data.plans)
      : (Object.keys(PLAN_CATALOG) as PlanTier[]).map((tier) => PLAN_CATALOG[tier]);

  const confirmUpgrade = async () => {
    if (!pendingTier) return;
    setError(null);
    try {
      await setPlan(pendingTier);
      setPendingTier(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  if (!entitlements) return null;

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-xl font-semibold m-0">{t("service.plan.compareTitle")}</h2>
        <div className="console-billing-toggle" role="group" aria-label={t("service.plan.billingCycleLabel")}>
          <button
            type="button"
            className={billingCycle === "monthly" ? "active" : ""}
            onClick={() => setBillingCycle("monthly")}
          >
            {t("service.billing.cycle.monthly")}
          </button>
          <button
            type="button"
            className={billingCycle === "yearly" ? "active" : ""}
            onClick={() => setBillingCycle("yearly")}
          >
            {t("service.billing.cycle.yearly")}
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <CurrentPlanSummary entitlements={entitlements} />
        <UpgradeValueCard />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
        {catalogEntries.map((entry) => (
          <PlanTierCard
            key={entry.tier}
            catalog={entry}
            current={entitlements.plan}
            billingCycle={billingCycle}
            highlighted={Boolean(entry.featured)}
            promoLabel={entry.promoLabel}
            onSelect={setPendingTier}
          />
        ))}
      </div>

      {!devMock && pendingTier != null && pendingTier !== "team" && (
        <p className="text-sm text-secondary m-0">{t("service.plan.checkoutComingSoon")}</p>
      )}

      <ModalOverlay open={pendingTier != null} onClose={() => setPendingTier(null)} labelledBy="upgrade-modal-title">
        <div className="glass-modal rounded-xl p-6 max-w-md">
          <h2 id="upgrade-modal-title" className="text-lg font-semibold m-0 mb-2">
            {pendingTier === "team"
              ? t("service.plan.teamModalTitle")
              : t("service.plan.upgradeModalTitle")}
          </h2>
          <p className="text-sm text-secondary m-0 mb-4">
            {pendingTier === "team"
              ? t("service.plan.teamModalBody")
              : devMock
                ? t("service.plan.upgradeModalBody")
                : t("service.plan.checkoutComingSoon")}
          </p>
          {error && <p className="text-sm text-error m-0 mb-4">{error}</p>}
          <div className="flex flex-wrap gap-2 justify-end">
            <button type="button" className="dw-btn-secondary" onClick={() => setPendingTier(null)}>
              {t("service.plan.cancel")}
            </button>
            {pendingTier !== "team" && devMock && (
              <button type="button" className="dw-btn-primary" onClick={() => void confirmUpgrade()}>
                {t("service.plan.confirmMockUpgrade")}
              </button>
            )}
            {pendingTier === "team" && (
              <button type="button" className="dw-btn-primary" onClick={() => setPendingTier(null)}>
                {t("service.plan.contactTeam")}
              </button>
            )}
          </div>
        </div>
      </ModalOverlay>
    </div>
  );
}
