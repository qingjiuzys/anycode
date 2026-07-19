import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import {
  accountCloud,
  getAccountToken,
  resolveAccountApiBase,
  resolvePortalUrl,
  setAccountToken,
} from "@/api/client/accountCloud";
import type { CloudAuthUser, CloudOrgMember } from "@/api/types/accountCloud";
import type { PlanTier, ServiceEntitlements } from "@/api/types/service";
import { bundleToEntitlements } from "@/lib/planCatalog";
import { cloudLoginUrl } from "@/lib/cloudPlatform";
import { openExternal } from "@/lib/openExternal";
import { router } from "@/router";

const LINK_POLL_MS = 2_000;
const LINK_TIMEOUT_MS = 120_000;

type AccountCloudContextValue = {
  baseUrl: string | null;
  portalUrl: string | null;
  configured: boolean;
  authenticated: boolean;
  cloudLinked: boolean;
  user: CloudAuthUser | null;
  sessionEmail: string;
  sessionDisplayName: string;
  loading: boolean;
  linking: boolean;
  linkError: string | null;
  linkCloudAccount: () => Promise<void>;
  openPortalLogin: (path?: string) => void;
  logout: () => Promise<void>;
  entitlements: ServiceEntitlements | null;
  members: CloudOrgMember[];
  usageLoading: boolean;
  usageStats: Awaited<ReturnType<typeof api.usageMetrics>>["usage"] | undefined;
  usageByModel: Awaited<ReturnType<typeof api.usageMetrics>>["by_model"];
  setPlan: (tier: PlanTier) => Promise<void>;
  updateBillingContact: (patch: {
    email?: string;
    companyName?: string;
    taxId?: string;
  }) => Promise<void>;
  refresh: () => void;
};

const AccountCloudContext = createContext<AccountCloudContextValue | null>(null);

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function applyCloudSessionFromApi(): Promise<boolean> {
  const session = await api.cloudSession();
  if (session.linked && session.access_token) {
    setAccountToken(session.access_token);
  }
  return session.linked;
}

export function AccountCloudProvider({ children }: { children: ReactNode }) {
  const qc = useQueryClient();
  const [tokenVersion, setTokenVersion] = useState(0);
  const [linking, setLinking] = useState(false);
  const [linkError, setLinkError] = useState<string | null>(null);
  const linkAbortRef = useRef<AbortController | null>(null);

  const health = useQuery({
    queryKey: ["health"],
    queryFn: api.health,
    staleTime: 60_000,
  });

  const baseUrl = useMemo(
    () => resolveAccountApiBase(health.data?.account_api_url),
    [health.data?.account_api_url],
  );

  const portalUrl = useMemo(
    () => resolvePortalUrl(health.data ?? null),
    [health.data],
  );

  const configured = Boolean(baseUrl);

  const applyCloudSession = useCallback(() => {
    setTokenVersion((v) => v + 1);
    void api.cloudSyncModels().then(() => {
      void qc.invalidateQueries({ queryKey: ["models-registry"] });
      void qc.invalidateQueries({ queryKey: ["llm-config"] });
    });
    void qc.invalidateQueries({ queryKey: ["cloud-session"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-me"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-bundle"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-members"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-api-keys"] });
  }, [qc]);

  const cloudSession = useQuery({
    queryKey: ["cloud-session"],
    queryFn: api.cloudSession,
    staleTime: 10_000,
    refetchInterval: linking ? LINK_POLL_MS : false,
    refetchOnWindowFocus: true,
  });

  useEffect(() => {
    void applyCloudSessionFromApi().then((linked) => {
      if (linked) {
        setLinking(false);
        setLinkError(null);
        applyCloudSession();
      }
    });
  }, [applyCloudSession]);

  useEffect(() => {
    if (!cloudSession.data?.linked) return;
    setLinking(false);
    setLinkError(null);
    if (cloudSession.data.access_token) {
      setAccountToken(cloudSession.data.access_token);
    }
    applyCloudSession();
  }, [cloudSession.data?.linked, cloudSession.data?.access_token, applyCloudSession]);

  useEffect(() => {
    const onLinked = () => {
      void applyCloudSessionFromApi().then((linked) => {
        if (linked) {
          setLinking(false);
          setLinkError(null);
          applyCloudSession();
        }
      });
    };
    window.addEventListener("anycode-cloud-linked", onLinked);
    return () => window.removeEventListener("anycode-cloud-linked", onLinked);
  }, [applyCloudSession]);

  const me = useQuery({
    queryKey: ["account-cloud-me", baseUrl, tokenVersion],
    queryFn: () => accountCloud.me(baseUrl!),
    enabled: configured && Boolean(getAccountToken()),
    retry: false,
  });

  const bundle = useQuery({
    queryKey: ["account-cloud-bundle", baseUrl, tokenVersion],
    queryFn: () => accountCloud.getBundle(baseUrl!),
    enabled: configured && Boolean(getAccountToken()) && me.isSuccess,
    staleTime: 30_000,
  });

  const members = useQuery({
    queryKey: ["account-cloud-members", baseUrl, tokenVersion],
    queryFn: () => accountCloud.listMembers(baseUrl!),
    enabled: configured && Boolean(getAccountToken()) && me.isSuccess,
    staleTime: 60_000,
  });

  const usage = useQuery({
    queryKey: ["usage-metrics", 30],
    queryFn: () => api.usageMetrics(30),
    staleTime: 120_000,
  });

  const cloudKeys = useQuery({
    queryKey: ["account-cloud-api-keys", baseUrl, tokenVersion],
    queryFn: () => accountCloud.listApiKeys(baseUrl!),
    enabled: configured && Boolean(getAccountToken()) && me.isSuccess,
    staleTime: 30_000,
  });

  const apiKeyUsed = (cloudKeys.data?.keys ?? []).filter((k) => !k.revoked).length;
  const tokenUsed = bundle.data?.account?.entitlements?.tokens_used ?? 0;

  const entitlements = useMemo(() => {
    if (!bundle.data?.account) return null;
    const base = bundleToEntitlements(bundle.data.account, tokenUsed, apiKeyUsed);
    if (members.data?.members?.length) {
      base.organization.members = members.data.members.map((m) => ({
        id: m.id,
        name: m.name,
        email: m.email,
        role: m.role,
        status: m.status as "active" | "invited",
        lastActive: m.last_active,
      }));
    }
    return base;
  }, [bundle.data?.account, tokenUsed, apiKeyUsed, members.data?.members]);

  const refresh = useCallback(() => {
    void qc.invalidateQueries({ queryKey: ["account-cloud-me"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-bundle"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-members"] });
    void qc.invalidateQueries({ queryKey: ["account-cloud-api-keys"] });
  }, [qc]);

  const logoutMut = useMutation({
    mutationFn: async () => {
      const remoteBase = baseUrl;
      const hadToken = Boolean(getAccountToken());

      // Optimistic local logout — don't wait on remote.
      setAccountToken(null);
      qc.setQueryData(["cloud-session"], {
        linked: false,
        access_token: null,
        user_email: null,
        display_name: null,
      });

      try {
        await api.cloudUnlink();
      } catch {
        /* ignore */
      }

      if (remoteBase && hadToken) {
        void accountCloud.logout(remoteBase).catch(() => {
          /* ignore */
        });
      }
    },
    onSuccess: () => {
      setTokenVersion((v) => v + 1);
      void qc.invalidateQueries({ queryKey: ["cloud-session"] });
      void qc.invalidateQueries({ queryKey: ["auth-me"] });
      void qc.invalidateQueries({ queryKey: ["account-cloud-me"] });
      refresh();
      void router.navigate({ to: "/cloud-login", replace: true });
    },
  });

  const upgradeMut = useMutation({
    mutationFn: async (plan: PlanTier) => {
      if (!baseUrl) throw new Error("account service not configured");
      await accountCloud.upgrade(baseUrl, plan);
    },
    onSuccess: refresh,
  });

  const billingMut = useMutation({
    mutationFn: async (patch: { email?: string; companyName?: string; taxId?: string }) => {
      if (!baseUrl) throw new Error("account service not configured");
      await accountCloud.patchBillingContact(baseUrl, {
        email: patch.email,
        company_name: patch.companyName,
        tax_id: patch.taxId,
      });
    },
    onSuccess: refresh,
  });

  const openPortalLogin = useCallback(
    (path = "/login") => {
      const base = portalUrl ?? baseUrl;
      if (!base) return;
      const url = `${base.replace(/\/$/, "")}${path}`;
      void openExternal(url);
    },
    [portalUrl, baseUrl],
  );

  const linkCloudAccount = useCallback(async () => {
    linkAbortRef.current?.abort();
    const abort = new AbortController();
    linkAbortRef.current = abort;
    setLinking(true);
    setLinkError(null);
    try {
      let browserUrl = cloudLoginUrl(portalUrl ?? undefined);
      let deviceCode: string | null = null;
      try {
        const start = await api.cloudLinkStart();
        deviceCode = start.device_code;
        browserUrl = start.browser_url || start.verification_uri_complete || browserUrl;
      } catch (err) {
        console.warn("cloudLinkStart failed, opening portal login", err);
      }
      await openExternal(browserUrl);
      const deadline = Date.now() + LINK_TIMEOUT_MS;
      while (Date.now() < deadline) {
        if (abort.signal.aborted) return;
        await sleep(LINK_POLL_MS);
        if (deviceCode) {
          try {
            const poll = await api.cloudLinkPoll(deviceCode);
            if (poll.linked) {
              await qc.fetchQuery({ queryKey: ["cloud-session"], queryFn: api.cloudSession });
              await applyCloudSessionFromApi();
              setLinking(false);
              setLinkError(null);
              applyCloudSession();
              window.dispatchEvent(new CustomEvent("anycode-cloud-linked"));
              return;
            }
            if (poll.error) {
              throw new Error(poll.error);
            }
          } catch (err) {
            console.warn("cloudLinkPoll failed", err);
          }
        }
        if (await applyCloudSessionFromApi()) {
          setLinking(false);
          applyCloudSession();
          return;
        }
      }
      throw new Error("link_timeout");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setLinkError(message === "popup_blocked" ? "popup_blocked" : message);
      throw err;
    } finally {
      if (linkAbortRef.current === abort) {
        setLinking(false);
        linkAbortRef.current = null;
      }
    }
  }, [applyCloudSession, portalUrl]);

  const cloudLinked = Boolean(cloudSession.data?.linked);

  const sessionEmail = useMemo(() => {
    const fromSession = cloudSession.data?.user_email?.trim() ?? "";
    if (fromSession) return fromSession;
    return me.data?.user?.email?.trim() ?? "";
  }, [cloudSession.data?.user_email, me.data?.user?.email]);

  const sessionDisplayName = useMemo(() => {
    const fromSession = cloudSession.data?.display_name?.trim() ?? "";
    if (fromSession) return fromSession;
    if (sessionEmail) return sessionEmail.split("@")[0] || sessionEmail;
    return me.data?.user?.display_name?.trim() ?? "";
  }, [cloudSession.data?.display_name, sessionEmail, me.data?.user?.display_name]);

  const sessionUser = useMemo((): CloudAuthUser | null => {
    if (me.data?.user) return me.data.user;
    if (!cloudLinked) return null;
    return {
      id: "",
      email: sessionEmail,
      display_name: sessionDisplayName || sessionEmail || "Cloud",
      role: "",
      organization_id: "",
    };
  }, [me.data?.user, cloudLinked, sessionEmail, sessionDisplayName]);

  const value: AccountCloudContextValue = {
    baseUrl,
    portalUrl,
    configured,
    cloudLinked,
    authenticated: Boolean(me.data?.authenticated) || cloudLinked,
    user: sessionUser,
    sessionEmail,
    sessionDisplayName,
    loading:
      health.isLoading ||
      (configured && Boolean(getAccountToken()) && (me.isLoading || bundle.isLoading)),
    linking,
    linkError,
    linkCloudAccount,
    openPortalLogin,
    logout: async () => {
      await logoutMut.mutateAsync();
    },
    entitlements,
    members: members.data?.members ?? [],
    usageLoading: usage.isLoading,
    usageStats: usage.data?.usage,
    usageByModel: usage.data?.by_model ?? [],
    setPlan: async (tier) => {
      await upgradeMut.mutateAsync(tier);
    },
    updateBillingContact: async (patch) => {
      await billingMut.mutateAsync(patch);
    },
    refresh,
  };

  return <AccountCloudContext.Provider value={value}>{children}</AccountCloudContext.Provider>;
}

export function useAccountCloud() {
  const ctx = useContext(AccountCloudContext);
  if (!ctx) {
    throw new Error("useAccountCloud must be used within AccountCloudProvider");
  }
  return ctx;
}

/** Back-compat alias for service sections */
export function useServiceEntitlements() {
  return useAccountCloud();
}
