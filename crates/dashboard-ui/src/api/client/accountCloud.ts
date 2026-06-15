import type {
  CloudAccountBundle,
  CloudApiKey,
  CloudAuthResponse,
  CloudBillingContact,
  CloudMeResponse,
  CloudOrgMember,
  CloudSubscription,
} from "@/api/types/accountCloud";

const TOKEN_KEY = "anycode-account-token";

export function getAccountToken(): string | null {
  try {
    return sessionStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export function setAccountToken(token: string | null) {
  try {
    if (token) sessionStorage.setItem(TOKEN_KEY, token);
    else sessionStorage.removeItem(TOKEN_KEY);
  } catch {
    /* ignore */
  }
}

function joinUrl(base: string, path: string): string {
  const b = base.endsWith("/") ? base.slice(0, -1) : base;
  const p = path.startsWith("/") ? path : `/${path}`;
  return `${b}${p}`;
}

async function accountFetch<T>(
  base: string,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  if (!headers.has("Content-Type") && init.body) {
    headers.set("Content-Type", "application/json");
  }
  const token = getAccountToken();
  if (token) headers.set("Authorization", `Bearer ${token}`);

  const res = await fetch(joinUrl(base, path), { ...init, headers });
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`${res.status} ${path}: ${text}`);
  }
  return JSON.parse(text) as T;
}

export const accountCloud = {
  health: (base: string) => accountFetch<{ ok: boolean; service: string }>(base, "/health"),

  register: (base: string, body: { email: string; password: string; display_name: string }) =>
    accountFetch<CloudAuthResponse>(base, "/api/v1/auth/register", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  login: (base: string, body: { email: string; password: string }) =>
    accountFetch<CloudAuthResponse>(base, "/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  logout: (base: string) =>
    accountFetch<{ ok: boolean }>(base, "/api/v1/auth/logout", { method: "POST" }),

  me: (base: string) => accountFetch<CloudMeResponse>(base, "/api/v1/auth/me"),

  getBundle: (base: string) =>
    accountFetch<{ account: CloudAccountBundle }>(base, "/api/v1/account/bundle"),

  upgrade: (base: string, plan: string) =>
    accountFetch<{ subscription: CloudSubscription }>(base, "/api/v1/account/subscription/upgrade", {
      method: "POST",
      body: JSON.stringify({ plan }),
    }),

  patchBillingContact: (base: string, patch: Partial<CloudBillingContact>) =>
    accountFetch<{ contact: CloudBillingContact }>(base, "/api/v1/account/billing/contact", {
      method: "PATCH",
      body: JSON.stringify(patch),
    }),

  listApiKeys: (base: string) =>
    accountFetch<{ keys: CloudApiKey[] }>(base, "/api/v1/account/api-keys"),

  createApiKey: (base: string, body: { name: string; expires_days?: number }) =>
    accountFetch<{ key: CloudApiKey; plaintext: string }>(base, "/api/v1/account/api-keys", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  revokeApiKey: (base: string, keyId: string) =>
    accountFetch<{ ok: boolean }>(base, `/api/v1/account/api-keys/${encodeURIComponent(keyId)}`, {
      method: "DELETE",
    }),

  listMembers: (base: string) =>
    accountFetch<{ members: CloudOrgMember[] }>(base, "/api/v1/org/members"),
};

export function resolveAccountApiBase(healthUrl?: string | null): string | null {
  const fromEnv = import.meta.env.VITE_ACCOUNT_API_URL?.trim();
  if (fromEnv) return fromEnv.replace(/\/$/, "");
  const fromHealth = healthUrl?.trim();
  if (fromHealth) return fromHealth.replace(/\/$/, "");
  return null;
}
