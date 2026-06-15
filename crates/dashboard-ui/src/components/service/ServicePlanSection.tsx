import { useState } from "react";
import type { PlanTier } from "@/api/types/service";
import { PLAN_CATALOG } from "@/lib/planCatalog";
import { CurrentPlanSummary, UpgradeValueCard } from "@/components/service/CurrentPlanSummary";
import { PlanTierCard } from "@/components/service/PlanTierCard";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServicePlanSection() {
  const t = useT();
  const { entitlements, setPlan } = useAccountCloud();
  const [pendingTier, setPendingTier] = useState<PlanTier | null>(null);
  const [error, setError] = useState<string | null>(null);

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
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <CurrentPlanSummary entitlements={entitlements} />
        <UpgradeValueCard />
      </div>

      <div>
        <h3 className="text-base font-semibold mb-3">{t("service.plan.compareTitle")}</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {(Object.keys(PLAN_CATALOG) as PlanTier[]).map((tier) => (
            <PlanTierCard
              key={tier}
              catalog={PLAN_CATALOG[tier]}
              current={entitlements.plan}
              highlighted={tier === "pro"}
              onSelect={setPendingTier}
            />
          ))}
        </div>
      </div>

      <ModalOverlay open={pendingTier != null} onClose={() => setPendingTier(null)} labelledBy="upgrade-modal-title">
        <div className="dw-section-card p-6">
          <h2 id="upgrade-modal-title" className="text-lg font-semibold m-0 mb-2">
            {pendingTier === "team"
              ? t("service.plan.teamModalTitle")
              : t("service.plan.upgradeModalTitle")}
          </h2>
          <p className="text-sm text-secondary m-0 mb-4">
            {pendingTier === "team"
              ? t("service.plan.teamModalBody")
              : t("service.plan.upgradeModalBody")}
          </p>
          {error && <p className="text-sm text-error m-0 mb-4">{error}</p>}
          <div className="flex flex-wrap gap-2 justify-end">
            <button type="button" className="dw-btn-secondary" onClick={() => setPendingTier(null)}>
              {t("service.plan.cancel")}
            </button>
            {pendingTier !== "team" && (
              <button type="button" className="dw-btn-primary" onClick={() => void confirmUpgrade()}>
                {t("service.plan.confirmMockUpgrade")}
              </button>
            )}
          </div>
        </div>
      </ModalOverlay>
    </div>
  );
}
