export type PlanTier = "free" | "cloud_5h" | "pro" | "team";

export interface CloudAuthUser {
  id: string;
  email: string;
  display_name: string;
  role: string;
  organization_id: string;
}

export interface CloudOrganization {
  id: string;
  name: string;
  plan_tier: string;
  sso_status: string;
}

export interface CloudSubscription {
  plan: string;
  status: string;
  billing_cycle: string;
  period_start: string;
  period_end: string;
  days_remaining: number;
  payment_method_bound: boolean;
}

export interface CloudEntitlements {
  token_limit: number;
  tokens_used: number;
  api_key_limit: number;
  seat_limit: number;
  seat_used: number;
}

export interface CloudBillingContact {
  email: string;
  company_name: string;
  tax_id: string;
}

export interface CloudInvoice {
  id: string;
  number: string;
  period_start: string;
  period_end: string;
  amount_fen?: number;
  currency?: "CNY";
  /** Legacy account-service compatibility only. */
  amount_cny?: number | null;
  status: string;
}

export interface CloudOrgMember {
  id: string;
  name: string;
  email: string;
  role: string;
  status: string;
  last_active: string;
}

export interface CloudApiKey {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  created_at: string;
  expires_at?: string | null;
  last_used_at?: string | null;
  revoked: boolean;
}

export interface CloudAccountBundle {
  user: CloudAuthUser;
  organization: CloudOrganization;
  subscription: CloudSubscription;
  entitlements: CloudEntitlements;
  billing_contact: CloudBillingContact;
  invoices: CloudInvoice[];
}

export interface CloudAuthResponse {
  user: CloudAuthUser;
  token: string;
  authenticated: boolean;
}

export interface CloudMeResponse {
  user: CloudAuthUser;
  authenticated: boolean;
}

export type PaymentProvider = "wechat" | "stripe";

export interface PaymentOrder {
  id: string;
  provider: string;
  plan: string;
  billing_cycle: string;
  amount_fen: number;
  currency: string;
  status: string;
  code_url?: string | null;
  expires_at: string;
  paid_at?: string | null;
}

export type CheckoutResponse =
  | { provider: "stripe"; checkout_url: string }
  | { provider: "wechat"; order: PaymentOrder };
