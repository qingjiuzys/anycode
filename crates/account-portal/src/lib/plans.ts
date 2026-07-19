import { useEffect, useState } from "react";
import { api, type CloudPlanCatalogEntry } from "../api";
import { useLocale, useT } from "../i18n/context";
import { formatMoney } from "./money";

export const PLAN_IDS = ["free", "cloud_5h", "pro", "team"] as const;

export type PlanId = (typeof PLAN_IDS)[number];

export type PlanTier = {
  id: string;
  name: string;
  price: string;
  desc: string;
  highlights: string[];
  featured: boolean;
  promoLabel: string | null;
};

function isKnownPlanId(id: string): id is PlanId {
  return (PLAN_IDS as readonly string[]).includes(id);
}

function formatPlanPrice(fen: number, locale: string): string {
  if (fen <= 0) return locale.startsWith("zh") ? "¥0" : "¥0";
  const money = formatMoney(fen, locale.startsWith("zh") ? "zh-CN" : "en-CN");
  return locale.startsWith("zh") ? `${money}/月` : `${money}/mo`;
}

function highlightsFor(plan: CloudPlanCatalogEntry, t: (key: string) => string): string[] {
  if (isKnownPlanId(plan.id)) {
    return [
      t(`plans.tiers.${plan.id}.h0`),
      t(`plans.tiers.${plan.id}.h1`),
      t(`plans.tiers.${plan.id}.h2`),
    ];
  }
  return [
    `${plan.token_limit.toLocaleString()} tokens`,
    `${plan.api_key_limit} API keys`,
    `${plan.seat_limit} seats`,
  ];
}

export function usePlanTiers(): { plans: PlanTier[]; loading: boolean } {
  const t = useT();
  const locale = useLocale();
  const [plans, setPlans] = useState<PlanTier[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void api
      .plansCatalog()
      .then((res) => {
        if (cancelled) return;
        const mapped = res.plans.map((p) => {
          const known = isKnownPlanId(p.id);
          return {
            id: p.id,
            name: known ? t(`plans.tiers.${p.id}.name`) : p.display_name,
            price: formatPlanPrice(p.monthly_price_fen, locale),
            desc: known
              ? t(`plans.tiers.${p.id}.desc`)
              : p.description ?? "",
            highlights: highlightsFor(p, t),
            featured: p.featured,
            promoLabel: p.promo_label,
          } satisfies PlanTier;
        });
        setPlans(mapped);
      })
      .catch(() => {
        if (cancelled) return;
        // Fallback: i18n-only (prices may drift until catalog is reachable).
        setPlans(
          PLAN_IDS.map((id) => ({
            id,
            name: t(`plans.tiers.${id}.name`),
            price: t(`plans.tiers.${id}.price`),
            desc: t(`plans.tiers.${id}.desc`),
            highlights: [
              t(`plans.tiers.${id}.h0`),
              t(`plans.tiers.${id}.h1`),
              t(`plans.tiers.${id}.h2`),
            ],
            featured: id === "pro",
            promoLabel: id === "pro" ? t("common.recommended") : null,
          })),
        );
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [t, locale]);

  return { plans, loading };
}
