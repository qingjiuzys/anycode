const TOKEN_KEY = "anycode_ops_token";

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(init?.headers as Record<string, string> | undefined),
  };
  if (token) headers.Authorization = `Bearer ${token}`;
  const res = await fetch(`/api/v1${path}`, { ...init, headers });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(body || res.statusText);
  }
  return res.json() as Promise<T>;
}

export async function login(email: string, password: string) {
  return api<{ token: string }>("/admin/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

export async function fetchMe() {
  return api<{ user: { email: string; role: string } }>("/admin/me");
}

export async function fetchAccounts() {
  return api<{ accounts: UpstreamAccount[] }>("/admin/upstream-accounts");
}

export async function createAccount(input: {
  name: string;
  api_key: string;
  base_url?: string;
  provider_id?: string;
  weight?: number;
}) {
  return api<{ account: UpstreamAccount }>("/admin/upstream-accounts", {
    method: "POST",
    body: JSON.stringify({
      provider_id: input.provider_id ?? "deepseek",
      name: input.name,
      api_key: input.api_key,
      base_url: input.base_url,
      weight: input.weight ?? 100,
    }),
  });
}

export async function patchAccount(
  id: string,
  patch: { status?: string; weight?: number },
) {
  return api<{ ok: boolean }>(`/admin/upstream-accounts/${id}`, {
    method: "PATCH",
    body: JSON.stringify(patch),
  });
}

export async function fetchHealthEvents() {
  return api<{ events: HealthEvent[] }>("/admin/health-events");
}

export async function fetchModels() {
  return api<{ models: CloudModel[] }>("/admin/models");
}

export async function fetchUsageOverview() {
  return api<{ usage: UsageRow[] }>("/admin/usage-overview");
}

export async function fetchPlans() {
  return api<{ plans: CloudPlan[] }>("/admin/plans");
}

export async function patchPlan(id: string, patch: Partial<CloudPlanPatch>) {
  return api<{ plan: CloudPlan }>(`/admin/plans/${id}`, {
    method: "PATCH",
    body: JSON.stringify(patch),
  });
}

export async function fetchIdentityReviews() {
  return api<{ reviews: IdentityReview[] }>("/admin/identity-reviews");
}

export async function approveIdentity(id: string) {
  return api<{ ok: boolean }>(`/admin/identity-reviews/${id}/approve`, { method: "POST" });
}

export async function rejectIdentity(id: string, reason: string) {
  return api<{ ok: boolean }>(`/admin/identity-reviews/${id}/reject`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}

export async function revealIdentity(id: string, purpose: string) {
  return api<{ legal_name: string; id_number: string }>(
    `/admin/identity-reviews/${id}/reveal`,
    { method: "POST", body: JSON.stringify({ purpose }) },
  );
}

export type UpstreamAccount = {
  id: string;
  provider_id: string;
  name: string;
  status: string;
  weight: number;
  failure_count: number;
  cooldown_until: string | null;
  has_active_key: boolean;
};

export type HealthEvent = {
  id: string;
  account_id: string;
  event_type: string;
  status_code: number | null;
  message: string | null;
  created_at: string;
};

export type CloudModel = {
  id: string;
  display_name: string;
  provider_id: string;
  min_plan: string;
  available: boolean;
};

export type UsageRow = {
  organization_id: string;
  total_tokens: number;
};

export type CloudPlan = {
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
  hosted_models_enabled: boolean;
  promo_label: string | null;
  featured: boolean;
  enabled: boolean;
  sort_order: number;
};

export type CloudPlanPatch = {
  display_name?: string;
  description?: string | null;
  monthly_price_fen?: number;
  yearly_price_fen?: number;
  token_limit?: number;
  api_key_limit?: number;
  seat_limit?: number;
  quota_window_secs?: number;
  calls_per_window?: number;
  hosted_models_enabled?: boolean;
  promo_label?: string | null;
  featured?: boolean;
  enabled?: boolean;
  sort_order?: number;
};

export type IdentityReview = {
  id: string;
  email: string;
  status: string;
  id_number_masked: string;
  submitted_at: string;
  reviewed_at: string | null;
  rejection_reason: string | null;
  document_upload_supported: false;
};
