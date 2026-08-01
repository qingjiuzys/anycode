import type {
  CheckoutResponse,
  CloudAccountBundle,
  CloudApiKey,
  CloudAuthResponse,
  CloudBillingContact,
  CloudMeResponse,
  CloudOrgMember,
  CloudSubscription,
  PaymentOrder,
  PaymentProvider,
} from "@/api/types/accountCloud";
import { CLOUD_PLATFORM } from "@/lib/cloudPlatform";
import { resolveApiBase } from "@/api/http";

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

/** Prefer workbench proxy so Tauri / local UI avoids CORS + WebKit "Load failed". */
function shouldUseWorkbenchProxy(): boolean {
  if (typeof window === "undefined") return false;
  if ("__TAURI_INTERNALS__" in window) return true;
  const { hostname, port } = window.location;
  const loopback = hostname === "127.0.0.1" || hostname === "localhost";
  return loopback && (port === "43180" || port === "43199" || port === "");
}

function resolveFetchTarget(base: string, path: string): { url: string; credentials: RequestCredentials } {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  if (shouldUseWorkbenchProxy() && normalizedPath.startsWith("/api/v1/")) {
    const workbench = resolveApiBase();
    return {
      url: joinUrl(workbench, `/api/cloud/upstream${normalizedPath}`),
      credentials: "include",
    };
  }
  return {
    url: joinUrl(base, normalizedPath),
    credentials: "omit",
  };
}

async function accountFetch<T>(
  base: string,
  path: string,
  init: RequestInit = {},
  timeoutMs = 15_000,
): Promise<T> {
  const headers = new Headers(init.headers);
  if (!headers.has("Content-Type") && init.body) {
    headers.set("Content-Type", "application/json");
  }
  const token = getAccountToken();
  if (token) headers.set("Authorization", `Bearer ${token}`);

  const { url, credentials } = resolveFetchTarget(base, path);
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, {
      ...init,
      headers,
      credentials,
      signal: init.signal ?? controller.signal,
    });
    const text = await res.text();
    if (!res.ok) {
      let detail = text;
      try {
        const parsed = JSON.parse(text) as { error?: string };
        if (typeof parsed.error === "string" && parsed.error.trim()) {
          detail = parsed.error.trim();
        }
      } catch {
        /* keep raw */
      }
      throw new Error(`${res.status} ${path}: ${detail}`);
    }
    if (!text.trim()) {
      return {} as T;
    }
    return JSON.parse(text) as T;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw new Error(`Request timed out after ${Math.round(timeoutMs / 1000)}s: ${path}`);
    }
    if (error instanceof TypeError) {
      throw new Error(`Load failed: ${path}`);
    }
    throw error;
  } finally {
    window.clearTimeout(timer);
  }
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
    accountFetch<{ ok: boolean }>(base, "/api/v1/auth/logout", { method: "POST" }, 5_000),

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

  plansCatalog: (base: string) =>
    accountFetch<{
      plans: Array<{
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
      }>;
    }>(base, "/api/v1/plans/catalog"),

  checkout: (
    base: string,
    plan: string,
    provider: PaymentProvider = "wechat",
    cycle: "monthly" | "yearly" = "monthly",
  ) =>
    accountFetch<CheckoutResponse>(base, "/api/v1/billing/checkout", {
      method: "POST",
      body: JSON.stringify({ plan, provider, cycle }),
    }),

  getPaymentOrder: (base: string, orderId: string) =>
    accountFetch<{ order: PaymentOrder }>(
      base,
      `/api/v1/billing/orders/${encodeURIComponent(orderId)}`,
    ),

  syncPaymentOrder: (base: string, orderId: string) =>
    accountFetch<{ order: PaymentOrder; synced: boolean }>(
      base,
      `/api/v1/billing/orders/${encodeURIComponent(orderId)}/sync`,
      { method: "POST" },
    ),
};

export function resolveAccountApiBase(healthUrl?: string | null): string {
  const fromEnv = import.meta.env.VITE_ACCOUNT_API_URL?.trim();
  if (fromEnv) return fromEnv.replace(/\/$/, "");
  const fromHealth = healthUrl?.trim();
  if (fromHealth) return fromHealth.replace(/\/$/, "");
  return CLOUD_PLATFORM.accountApiUrl;
}

export function resolvePortalUrl(
  health?: { account_portal_url?: string | null; account_api_url?: string | null } | null,
): string {
  const fromEnv = import.meta.env.VITE_ACCOUNT_PORTAL_URL?.trim();
  if (fromEnv) return fromEnv.replace(/\/$/, "");
  const fromHealth = health?.account_portal_url?.trim() || health?.account_api_url?.trim();
  if (fromHealth) return fromHealth.replace(/\/$/, "");
  return CLOUD_PLATFORM.portalUrl;
}
