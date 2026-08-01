import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { BillingCycle, PlanCatalogEntry, PlanTier } from "@/api/types/service";
import type { PaymentOrder } from "@/api/types/accountCloud";
import { accountCloud } from "@/api/client/accountCloud";
import { PLAN_CATALOG, catalogFromApi } from "@/lib/planCatalog";
import { isDevMockEnabled } from "@/lib/isDevMockEnabled";
import { openExternal } from "@/lib/openExternal";
import { CurrentPlanSummary } from "@/components/service/CurrentPlanSummary";
import { PlanTierCard } from "@/components/service/PlanTierCard";
import { WeChatPayModal } from "@/components/service/WeChatPayModal";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useLocale, useT } from "@/i18n/context";

export function ServicePlanSection() {
  const t = useT();
  const locale = useLocale();
  const { entitlements, setPlan, baseUrl, openPortalLogin, refresh } = useAccountCloud();
  const [billingCycle, setBillingCycle] = useState<BillingCycle>("monthly");
  const [pendingTier, setPendingTier] = useState<PlanTier | null>(null);
  const [checkoutLoading, setCheckoutLoading] = useState<PlanTier | null>(null);
  const [wechatOrder, setWechatOrder] = useState<PaymentOrder | null>(null);
  const [error, setError] = useState<string | null>(null);
  const devMock = isDevMockEnabled();
  const checkoutCta =
    locale === "zh" ? t("service.plan.wechatSubscribe") : t("service.plan.stripeSubscribe");

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

  const handleCheckout = async (tier: PlanTier) => {
    if (!baseUrl || tier === "free" || tier === "team") return;
    if (devMock) {
      setPendingTier(tier);
      return;
    }
    setError(null);
    setCheckoutLoading(tier);
    try {
      const provider = locale === "zh" ? "wechat" : "stripe";
      const res = await accountCloud.checkout(baseUrl, tier, provider, billingCycle);
      if (res.provider === "stripe" && res.checkout_url) {
        await openExternal(res.checkout_url);
        return;
      }
      if (res.provider === "wechat" && res.order) {
        setWechatOrder(res.order);
        return;
      }
      setError(t("service.plan.checkoutError"));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setCheckoutLoading(null);
    }
  };

  if (!entitlements) return null;

  return (
    <div className="space-y-4">
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

      <div className="w-full lg:max-w-[50%]">
        <CurrentPlanSummary entitlements={entitlements} />
      </div>

      {error && (
        <p className="text-sm text-error m-0" role="alert">
          {error}
        </p>
      )}

      <div className="console-plan-grid grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
        {catalogEntries.map((entry) => (
          <PlanTierCard
            key={entry.tier}
            catalog={entry}
            current={entitlements.plan}
            billingCycle={billingCycle}
            highlighted={Boolean(entry.featured)}
            promoLabel={entry.promoLabel}
            checkoutCta={checkoutCta}
            checkoutLoading={checkoutLoading}
            onCheckout={handleCheckout}
            onContactTeam={() => openPortalLogin("/console/plans")}
          />
        ))}
      </div>

      {!devMock && (
        <p className="text-xs text-secondary m-0">
          {t("service.plan.portalFallback")}{" "}
          <button
            type="button"
            className="text-accent underline-offset-2 hover:underline"
            onClick={() => openPortalLogin("/console/plans")}
          >
            {t("service.plan.checkoutPortalCta")}
          </button>
        </p>
      )}

      <ModalOverlay open={pendingTier != null} onClose={() => setPendingTier(null)} labelledBy="upgrade-modal-title" zIndex={360}>
        <div className="glass-modal rounded-xl p-6 max-w-md">
          <h2 id="upgrade-modal-title" className="text-lg font-semibold m-0 mb-2">
            {t("service.plan.upgradeModalTitle")}
          </h2>
          <p className="text-sm text-secondary m-0 mb-4">{t("service.plan.upgradeModalBody")}</p>
          {error && <p className="text-sm text-error m-0 mb-4">{error}</p>}
          <div className="flex flex-wrap gap-2 justify-end">
            <button type="button" className="dw-btn-secondary" onClick={() => setPendingTier(null)}>
              {t("service.plan.cancel")}
            </button>
            <button type="button" className="dw-btn-primary" onClick={() => void confirmUpgrade()}>
              {t("service.plan.confirmMockUpgrade")}
            </button>
          </div>
        </div>
      </ModalOverlay>

      {wechatOrder && baseUrl && (
        <WeChatPayModal
          baseUrl={baseUrl}
          order={wechatOrder}
          onClose={() => setWechatOrder(null)}
          onPaid={() => {
            setWechatOrder(null);
            refresh();
          }}
        />
      )}
    </div>
  );
}
