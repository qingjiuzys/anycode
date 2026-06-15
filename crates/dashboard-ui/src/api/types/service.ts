export type PlanTier = "free" | "pro" | "team";

export type SubscriptionStatus = "active" | "trialing" | "past_due" | "canceled";

export type BillingCycle = "monthly" | "yearly";

export type InvoiceStatus = "paid" | "pending" | "draft";

export type SsoStatus = "disabled" | "configured" | "pending";

export type MemberStatus = "active" | "invited";

export interface ServiceQuota {
  tokenLimit: number;
  tokenUsed: number;
  apiKeyLimit: number;
  apiKeyUsed: number;
  seatLimit: number;
  seatUsed: number;
}

export interface BillingPeriod {
  start: string;
  end: string;
  daysRemaining: number;
}

export interface BillingContact {
  email: string;
  companyName: string;
  taxId: string;
}

export interface OrgMember {
  id: string;
  name: string;
  email: string;
  role: string;
  status: MemberStatus;
  lastActive: string;
}

export interface ServiceOrganization {
  name: string;
  members: OrgMember[];
  ssoStatus: SsoStatus;
}

export interface ServiceInvoice {
  id: string;
  number: string;
  periodStart: string;
  periodEnd: string;
  amountUsd: number;
  status: InvoiceStatus;
}

export interface ServiceEntitlements {
  plan: PlanTier;
  subscriptionStatus: SubscriptionStatus;
  billingCycle: BillingCycle;
  quota: ServiceQuota;
  billingPeriod: BillingPeriod;
  billingContact: BillingContact;
  organization: ServiceOrganization;
  invoices: ServiceInvoice[];
  paymentMethodBound: boolean;
}

export interface PlanCatalogEntry {
  tier: PlanTier;
  monthlyPriceUsd: number;
  yearlyPriceUsd: number;
  tokenLimit: number;
  apiKeyLimit: number;
  seatLimit: number;
  featureKeys: readonly string[];
}
