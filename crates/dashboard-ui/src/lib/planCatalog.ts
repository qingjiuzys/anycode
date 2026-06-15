import type { PlanCatalogEntry, PlanTier, ServiceEntitlements } from "@/api/types/service";
import type { CloudAccountBundle } from "@/api/types/accountCloud";

export const PLAN_CATALOG: Record<PlanTier, PlanCatalogEntry> = {
  free: {
    tier: "free",
    monthlyPriceUsd: 0,
    yearlyPriceUsd: 0,
    tokenLimit: 500_000,
    apiKeyLimit: 1,
    seatLimit: 1,
    featureKeys: [
      "service.plan.features.localWorkbench",
      "service.plan.features.basicUsage",
      "service.plan.features.singleApiKey",
    ],
  },
  pro: {
    tier: "pro",
    monthlyPriceUsd: 29,
    yearlyPriceUsd: 290,
    tokenLimit: 5_000_000,
    apiKeyLimit: 5,
    seatLimit: 1,
    featureKeys: [
      "service.plan.features.higherQuota",
      "service.plan.features.automation",
      "service.plan.features.apiAccess",
      "service.plan.features.prioritySupport",
    ],
  },
  team: {
    tier: "team",
    monthlyPriceUsd: 99,
    yearlyPriceUsd: 990,
    tokenLimit: 20_000_000,
    apiKeyLimit: 20,
    seatLimit: 10,
    featureKeys: [
      "service.plan.features.teamSeats",
      "service.plan.features.rbac",
      "service.plan.features.audit",
      "service.plan.features.ssoPlaceholder",
      "service.plan.features.teamBilling",
    ],
  },
};

export function bundleToEntitlements(
  bundle: CloudAccountBundle,
  tokenUsed: number,
  apiKeyUsed: number,
): ServiceEntitlements {
  const plan = (bundle.subscription.plan as PlanTier) || "free";
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
      amountUsd: inv.amount_usd,
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
