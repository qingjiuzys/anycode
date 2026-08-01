import type { PlanCatalogEntry, PlanTier, ServiceEntitlements } from "@/api/types/service";
import type { CloudAccountBundle } from "@/api/types/accountCloud";

/** Fallback catalog when `/api/v1/plans/catalog` is unreachable (seed-aligned). */
export const PLAN_CATALOG: Record<PlanTier, PlanCatalogEntry> = {
  free: {
    tier: "free",
    monthlyPriceFen: 0,
    yearlyPriceFen: 0,
    tokenLimit: 20_000_000,
    apiKeyLimit: 1,
    seatLimit: 1,
    featured: false,
    promoLabel: null,
    featureKeys: [
      "service.plan.features.localWorkbench",
      "service.plan.features.basicUsage",
      "service.plan.features.singleApiKey",
    ],
  },
  cloud_5h: {
    tier: "cloud_5h",
    monthlyPriceFen: 9_900,
    yearlyPriceFen: 99_000,
    tokenLimit: 50_000_000,
    apiKeyLimit: 3,
    seatLimit: 1,
    featured: false,
    promoLabel: null,
    featureKeys: [
      "service.plan.features.cloud5hWindow",
      "service.plan.features.cloud5hCalls",
      "service.plan.features.apiAccess",
    ],
  },
  pro: {
    tier: "pro",
    monthlyPriceFen: 59_900,
    yearlyPriceFen: 599_000,
    tokenLimit: 15_000_000,
    apiKeyLimit: 5,
    seatLimit: 1,
    featured: true,
    promoLabel: null,
    featureKeys: [
      "service.plan.features.cloud5hWindow",
      "service.plan.features.proWindowCalls",
      "service.plan.features.apiAccess",
      "service.plan.features.flashOnlyHosted",
    ],
  },
  team: {
    tier: "team",
    monthlyPriceFen: 199_900,
    yearlyPriceFen: 1_999_000,
    tokenLimit: 15_000_000_000,
    apiKeyLimit: 20,
    seatLimit: 10,
    featured: false,
    promoLabel: null,
    featureKeys: [
      "service.plan.features.teamSeats",
      "service.plan.features.rbac",
      "service.plan.features.audit",
      "service.plan.features.ssoPlaceholder",
      "service.plan.features.teamBilling",
    ],
  },
};

const FEATURE_KEYS_BY_TIER: Record<PlanTier, readonly string[]> = {
  free: PLAN_CATALOG.free.featureKeys,
  cloud_5h: PLAN_CATALOG.cloud_5h.featureKeys,
  pro: PLAN_CATALOG.pro.featureKeys,
  team: PLAN_CATALOG.team.featureKeys,
};

export type CloudPlanCatalogDto = {
  id: string;
  monthly_price_fen: number;
  yearly_price_fen: number;
  token_limit: number;
  api_key_limit: number;
  seat_limit: number;
  promo_label: string | null;
  featured: boolean;
  enabled: boolean;
  sort_order: number;
};

export function isPlanTier(id: string): id is PlanTier {
  return id === "free" || id === "pro" || id === "team" || id === "cloud_5h";
}

export function catalogFromApi(rows: CloudPlanCatalogDto[]): PlanCatalogEntry[] {
  return rows
    .filter((r) => isPlanTier(r.id))
    .map((r) => {
      const tier = r.id as PlanTier;
      return {
        tier,
        monthlyPriceFen: r.monthly_price_fen,
        yearlyPriceFen: r.yearly_price_fen,
        tokenLimit: r.token_limit,
        apiKeyLimit: r.api_key_limit,
        seatLimit: r.seat_limit,
        featured: r.featured,
        promoLabel: r.promo_label,
        featureKeys: FEATURE_KEYS_BY_TIER[tier] ?? PLAN_CATALOG.free.featureKeys,
      };
    });
}

export function bundleToEntitlements(
  bundle: CloudAccountBundle,
  tokenUsed: number,
  apiKeyUsed: number,
): ServiceEntitlements {
  const plan = isPlanTier(bundle.subscription.plan)
    ? bundle.subscription.plan
    : "free";
  return {
    plan,
    subscriptionStatus: bundle.subscription.status as ServiceEntitlements["subscriptionStatus"],
    billingCycle: bundle.subscription.billing_cycle as ServiceEntitlements["billingCycle"],
    quota: {
      tokenLimit: bundle.entitlements.token_limit,
      tokenUsed,
      apiKeyLimit: bundle.entitlements.api_key_limit,
      apiKeyUsed,
      seatLimit: bundle.entitlements.seat_limit,
      seatUsed: bundle.entitlements.seat_used,
    },
    billingPeriod: {
      start: bundle.subscription.period_start,
      end: bundle.subscription.period_end,
      daysRemaining: bundle.subscription.days_remaining,
    },
    billingContact: {
      email: bundle.billing_contact.email,
      companyName: bundle.billing_contact.company_name,
      taxId: bundle.billing_contact.tax_id,
    },
    organization: {
      name: bundle.organization.name,
      members: bundle.user
        ? [
            {
              id: bundle.user.id,
              name: bundle.user.display_name,
              email: bundle.user.email,
              role: bundle.user.role,
              status: "active",
              lastActive: new Date().toISOString().slice(0, 10),
            },
          ]
        : [],
      ssoStatus: bundle.organization.sso_status as ServiceEntitlements["organization"]["ssoStatus"],
    },
    invoices: bundle.invoices.map((inv) => ({
      id: inv.id,
      number: inv.number,
      periodStart: inv.period_start,
      periodEnd: inv.period_end,
      amountFen: inv.amount_fen ?? Math.round((inv.amount_cny ?? 0) * 100),
      currency: "CNY",
      status: inv.status as ServiceEntitlements["invoices"][number]["status"],
    })),
    paymentMethodBound: bundle.subscription.payment_method_bound,
  };
}

export function quotaPercent(used: number, limit: number): number {
  if (limit <= 0) return 0;
  return Math.min(100, Math.round((used / limit) * 100));
}

export function isQuotaNearLimit(used: number, limit: number, threshold = 0.8): boolean {
  if (limit <= 0) return false;
  return used / limit >= threshold;
}
