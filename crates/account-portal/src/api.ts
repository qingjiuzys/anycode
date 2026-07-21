const TOKEN_KEY = "anycode-portal-token";

let unauthorizedHandler: (() => void) | null = null;

export function setUnauthorizedHandler(handler: (() => void) | null) {
  unauthorizedHandler = handler;
}

export function getToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export function setToken(token: string | null) {
  try {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    /* ignore */
  }
}

export type PaymentProvider = "wechat" | "stripe";

export type CloudPlanCatalogEntry = {
  id: string;
  display_name: string;
  description: string | null;
  monthly_price_fen: number;
  yearly_price_fen: number;
  token_limit: number;
  api_key_limit: number;
  seat_limit: number;
  quota_window_secs: number;
  calls_per_window: number;
  promo_label: string | null;
  featured: boolean;
  enabled: boolean;
  sort_order: number;
};

export type PaymentOrder = {
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
};

type CheckoutResponse =
  | { provider: "stripe"; checkout_url: string }
  | { provider: "wechat"; order: PaymentOrder };

async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (!headers.has("Content-Type") && init.body) {
    headers.set("Content-Type", "application/json");
  }
  const token = getToken();
  if (token) headers.set("Authorization", `Bearer ${token}`);

  const res = await fetch(path, { ...init, headers });
  const text = await res.text();
  if (res.status === 401) {
    unauthorizedHandler?.();
    throw new Error(`401: ${text || "session expired"}`);
  }
  if (!res.ok) throw new Error(`${res.status}: ${text}`);
  if (!text.trim()) return {} as T;
  return JSON.parse(text) as T;
}

export const api = {
  sendRegistrationCode: (email: string) =>
    apiFetch<{ ok: boolean; expires_in: number }>("/api/v1/auth/email/send-code", {
      method: "POST",
      body: JSON.stringify({ email }),
    }),

  register: (body: {
    email: string;
    password: string;
    display_name: string;
    verification_code: string;
    privacy_consent: boolean;
    consent_version: string;
  }) =>
    apiFetch<{ token: string }>("/api/v1/auth/register", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  login: (body: { email: string; password: string }) =>
    apiFetch<{ token: string }>("/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  logout: () => apiFetch<{ ok: boolean }>("/api/v1/auth/logout", { method: "POST" }),

  me: () =>
    apiFetch<{
      user: { email: string; display_name: string };
      identity_status?: string;
      authenticated?: boolean;
    }>("/api/v1/auth/me"),

  updateProfile: (body: { display_name: string }) =>
    apiFetch<{ ok: boolean }>("/api/v1/account/profile", {
      method: "PATCH",
      body: JSON.stringify(body),
    }),

  identityStatus: () =>
    apiFetch<{
      identity: {
        status: string;
        legal_name_masked: string | null;
        id_number_masked: string | null;
        rejection_reason: string | null;
        document_upload_supported: boolean;
      };
    }>("/api/v1/account/identity"),

  submitIdentity: (body: { legal_name: string; id_number: string }) =>
    apiFetch<{ ok: boolean; status: string }>("/api/v1/account/identity/submit", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  bundle: () =>
    apiFetch<{
      account: {
        subscription: {
          plan: string;
          status: string;
          billing_cycle?: string;
          period_start?: string;
          period_end?: string;
        };
        entitlements: {
          token_limit: number;
          tokens_used: number;
          api_key_limit?: number;
          seat_limit?: number;
          hosted_models_enabled: boolean;
        };
        invoices: Array<{
          number: string;
          amount_fen: number;
          currency: "CNY";
          status: string;
        }>;
      };
    }>("/api/v1/account/bundle"),

  upgrade: (plan: string) =>
    apiFetch("/api/v1/account/subscription/upgrade", {
      method: "POST",
      body: JSON.stringify({ plan }),
    }),

  checkout: (
    plan: string,
    provider: PaymentProvider = "wechat",
    cycle: "monthly" | "yearly" = "monthly",
  ) =>
    apiFetch<CheckoutResponse>("/api/v1/billing/checkout", {
      method: "POST",
      body: JSON.stringify({ plan, provider, cycle }),
    }),

  getPaymentOrder: (orderId: string) =>
    apiFetch<{ order: PaymentOrder }>(`/api/v1/billing/orders/${orderId}`),

  syncPaymentOrder: (orderId: string) =>
    apiFetch<{ order: PaymentOrder; synced: boolean }>(
      `/api/v1/billing/orders/${orderId}/sync`,
      { method: "POST" },
    ),

  models: () =>
    apiFetch<{
      models: Array<{
        id: string;
        display_name: string;
        provider_id: string;
        min_plan: string;
        available: boolean;
        price_per_1m_input_cny: number;
        price_per_1m_output_cny: number;
        currency: "CNY";
      }>;
    }>("/api/v1/models/catalog"),

  usage: () =>
    apiFetch<{
      usage: { tokens_used: number; token_limit: number; by_model: Array<{ model_id: string; total_tokens: number }> };
    }>("/api/v1/usage/summary"),

  plansCatalog: () =>
    apiFetch<{ plans: CloudPlanCatalogEntry[] }>("/api/v1/plans/catalog"),

  devices: () =>
    apiFetch<{
      devices: Array<{ id: string; device_name: string; last_seen_at: string; revoked: boolean }>;
    }>("/api/v1/devices"),

  deviceLinkStart: (device_name?: string) =>
    apiFetch<{
      device_code: string;
      deep_link: string;
      verification_uri: string;
    }>("/api/v1/devices/link/start", {
      method: "POST",
      body: JSON.stringify({ device_name }),
    }),

  approveDeviceLink: (device_code: string) =>
    apiFetch<{ ok: boolean }>("/api/v1/devices/link/approve", {
      method: "POST",
      body: JSON.stringify({ device_code }),
    }),

  revokeDevice: (id: string) =>
    apiFetch(`/api/v1/devices/${id}`, { method: "DELETE" }),

  listApiKeys: () =>
    apiFetch<{
      keys: Array<{
        id: string;
        name: string;
        prefix: string;
        created_at: string;
        expires_at: string | null;
        last_used_at: string | null;
        revoked: boolean;
      }>;
    }>("/api/v1/account/api-keys"),

  createApiKey: (body: { name: string; expires_days?: number }) =>
    apiFetch<{
      key: { id: string; name: string; prefix: string };
      plaintext: string;
    }>("/api/v1/account/api-keys", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  revokeApiKey: (id: string) =>
    apiFetch(`/api/v1/account/api-keys/${id}`, { method: "DELETE" }),
};
