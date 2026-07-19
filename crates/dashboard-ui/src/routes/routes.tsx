import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { Layout } from "@/components/Layout";
import type { SettingsSection } from "@/components/settings/SettingsNav";
import type { ServiceSection } from "@/components/service/ServiceNav";
import { api } from "@/api/client";
import {
  conversationSearchParams,
  conversationsCanonicalHref,
  parseConversationSearch,
} from "@/lib/conversationsSearch";
import {
  controlCenterRedirectTarget,
  shouldOpenControlCenterForLocation,
} from "@/lib/controlCenterPaths";
import { isOfflineWorkbenchAllowed } from "@/lib/offlineWorkbench";
import { CloudLoginPage } from "@/pages/CloudLoginPage";
import {
  AgentsPage,
  ArtifactDetailPage,
  AssetsPage,
  AuditPage,
  AutomationsPage,
  ConversationsPage,
  EventDetailPage,
  HomePage,
  OverviewPage,
  Page,
  ProjectDetailPage,
  ProjectsPage,
  ReportsPage,
  SessionDetailPage,
  SettingsPage,
  ServicePage,
  SkillDetailPage,
} from "@/routes/lazyPages";

export const rootRoute = createRootRoute({
  component: () => <Outlet />,
});

/** Dedicated frontmost gate — no workbench chrome. */
export const cloudLoginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/cloud-login",
  component: CloudLoginPage,
  beforeLoad: async () => {
    try {
      const cloud = await api.cloudSession();
      if (cloud.linked) {
        throw redirect({ to: "/conversations", replace: true });
      }
    } catch (e) {
      if (e && typeof e === "object" && "to" in e) throw e;
    }
  },
});

export const shellRoute = createRoute({
  id: "_shell",
  getParentRoute: () => rootRoute,
  component: Layout,
  beforeLoad: async ({ location }) => {
    // Cloud link preferred; offline workbench is an explicit local-first escape hatch.
    try {
      const cloud = await api.cloudSession();
      if (!cloud.linked && !isOfflineWorkbenchAllowed()) {
        throw redirect({ to: "/cloud-login" });
      }
    } catch (e) {
      if (e && typeof e === "object" && "to" in e) throw e;
      if (!isOfflineWorkbenchAllowed()) {
        throw redirect({ to: "/cloud-login" });
      }
    }
    try {
      const svc = await api.serviceStatus();
      if (!svc.service.loopback) {
        const me = await api.authMe();
        if (!me.authenticated && !isOfflineWorkbenchAllowed()) {
          throw redirect({ to: "/cloud-login" });
        }
      }
    } catch (e) {
      if (e && typeof e === "object" && "to" in e) throw e;
    }
    if (shouldOpenControlCenterForLocation(location.pathname, location.searchStr ?? "")) {
      throw redirect({
        ...controlCenterRedirectTarget(location.pathname, location.searchStr ?? ""),
        replace: true,
      });
    }
  },
});

export const loginRedirectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  beforeLoad: () => {
    throw redirect({ to: "/cloud-login" });
  },
});

export const setupRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/setup",
  validateSearch: (
    search: Record<string, unknown>,
  ): { step?: string; tab?: string; section?: SettingsSection } => {
    const sectionRaw = typeof search.section === "string" ? search.section.trim() : "";
    const valid = [
      "auth",
      "prefs",
      "data",
      "service",
      "model",
      "agents",
      "skills",
      "security",
      "notify",
      "channels",
      "gates",
      "plugins",
      "ops",
      "about",
    ] as const;
    const section =
      sectionRaw && (valid as readonly string[]).includes(sectionRaw)
        ? (sectionRaw as SettingsSection)
        : undefined;
    return {
      step: typeof search.step === "string" ? search.step : undefined,
      tab: typeof search.tab === "string" ? search.tab : undefined,
      section,
    };
  },
  beforeLoad: ({ search }) => {
    const step = search.step?.trim() ?? "";
    const tab = search.tab?.trim() ?? "";
    let section: SettingsSection = "model";
    if (step === "channels" || tab === "telegram" || tab === "discord" || tab === "wechat") {
      section = "channels";
    } else if (step === "memory") {
      section = "data";
    } else if (step === "skills") {
      section = "skills";
    } else if (search.section) {
      section = search.section;
    }
    throw redirect({
      to: "/settings",
      search: { section },
      replace: true,
    });
  },
});

export const indexRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  validateSearch: (search: Record<string, unknown>): { project?: string } => {
    const project =
      typeof search.project === "string" && search.project.trim()
        ? search.project.trim()
        : undefined;
    return { project };
  },
  component: () => (
    <Page>
      <HomePage />
    </Page>
  ),
});

export const overviewRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/overview",
  component: () => (
    <Page>
      <OverviewPage />
    </Page>
  ),
});

export const projectsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects",
  component: () => (
    <Page>
      <ProjectsPage />
    </Page>
  ),
});

export const projectDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects/$projectId",
  component: () => (
    <Page>
      <ProjectDetailPage />
    </Page>
  ),
});

export const conversationsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/conversations",
  beforeLoad: ({ location }) => {
    const href = conversationsCanonicalHref(location.searchStr ?? "");
    if (!href) return;
    const canon = conversationSearchParams(
      parseConversationSearch(href.split("?")[1] ?? ""),
    );
    throw redirect({
      to: "/conversations",
      search: () => canon,
      replace: true,
    });
  },
  validateSearch: (
    search: Record<string, unknown>,
  ): {
    project?: string;
    session?: string;
    agent?: string;
    filter?: string;
    cc?: string;
  } => {
    const project =
      typeof search.project === "string" && search.project.trim()
        ? search.project.trim()
        : undefined;
    const session =
      typeof search.session === "string" && search.session.trim()
        ? search.session.trim()
        : undefined;
    const agent =
      typeof search.agent === "string" && search.agent.trim()
        ? search.agent.trim()
        : undefined;
    const cc =
      typeof search.cc === "string" && search.cc.trim() ? search.cc.trim() : undefined;
    const base = { project, session, agent, cc };

    const f = typeof search.filter === "string" ? search.filter.trim() : "";
    if (f) return { ...base, filter: f };

    // Legacy URLs — infer a single `filter` (API fields derived in conversationsSearch.ts).
    const raw = new URLSearchParams();
    for (const [k, v] of Object.entries(search)) {
      if (v === undefined || v === null || v === "") continue;
      raw.set(k, String(v));
    }
    const inferred = parseConversationSearch(`?${raw.toString()}`).filter;
    return inferred ? { ...base, filter: inferred } : base;
  },
  component: () => (
    <Page>
      <ConversationsPage />
    </Page>
  ),
});

export const sessionDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/sessions/$sessionId",
  validateSearch: (
    search: Record<string, unknown>,
  ): { tab?: "debug" | "audit" } => {
    const tab = search.tab;
    if (tab === "debug" || tab === "audit") return { tab };
    return {};
  },
  beforeLoad: async ({ params, search }) => {
    if (search.tab === "debug" || search.tab === "audit") return;
    try {
      const data = await api.session(params.sessionId);
      throw redirect({
        to: "/conversations",
        search: conversationSearchParams({
          session: params.sessionId,
          project: data.session.project_id,
        }),
        replace: true,
      });
    } catch (e) {
      if (e && typeof e === "object" && "to" in e) throw e;
    }
  },
  component: () => (
    <Page>
      <SessionDetailPage />
    </Page>
  ),
});

export const eventDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/events/$eventId",
  component: () => (
    <Page>
      <EventDetailPage />
    </Page>
  ),
});

export const automationsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/automations",
  component: () => (
    <Page>
      <AutomationsPage />
    </Page>
  ),
});

export const assetsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/assets",
  validateSearch: (
    search: Record<string, unknown>,
  ): { trust?: "unverified" | "blocked" } => {
    const trust = search.trust;
    if (trust === "unverified" || trust === "blocked") {
      return { trust };
    }
    return {};
  },
  component: () => (
    <Page>
      <AssetsPage />
    </Page>
  ),
});

export const artifactDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/assets/$artifactId",
  component: () => (
    <Page>
      <ArtifactDetailPage />
    </Page>
  ),
});

export const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents",
  component: () => (
    <Page>
      <AgentsPage />
    </Page>
  ),
});

export const skillDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents/$skillId",
  component: () => (
    <Page>
      <SkillDetailPage />
    </Page>
  ),
});

export const reportsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/reports",
  validateSearch: (
    search: Record<string, unknown>,
  ): { project_id?: string; session_id?: string; artifact_id?: string } => {
    const str = (v: unknown) =>
      typeof v === "string" && v.trim() ? v.trim() : undefined;
    return {
      project_id: str(search.project_id),
      session_id: str(search.session_id),
      artifact_id: str(search.artifact_id),
    };
  },
  component: () => (
    <Page>
      <ReportsPage />
    </Page>
  ),
});

export const auditRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/audit",
  component: () => (
    <Page>
      <AuditPage />
    </Page>
  ),
});

export const accountRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/account",
  validateSearch: (
    search: Record<string, unknown>,
  ): { section?: ServiceSection } => {
    const section = search.section;
    const valid = ["overview", "plan", "usage", "billing", "api", "enterprise"] as const;
    if (typeof section === "string" && (valid as readonly string[]).includes(section)) {
      return { section: section as ServiceSection };
    }
    return {};
  },
  component: () => (
    <Page>
      <ServicePage />
    </Page>
  ),
});

export const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/settings",
  validateSearch: (
    search: Record<string, unknown>,
  ): { section?: SettingsSection } => {
    const section = search.section;
    const valid = [
      "prefs",
      "data",
      "service",
      "model",
      "agents",
      "skills",
      "security",
      "notify",
      "channels",
      "gates",
      "plugins",
      "ops",
      "about",
    ] as const;
    if (typeof section === "string" && (valid as readonly string[]).includes(section)) {
      return { section: section as SettingsSection };
    }
    return {};
  },
  component: () => (
    <Page>
      <SettingsPage />
    </Page>
  ),
});

export const routeTree = rootRoute.addChildren([
  cloudLoginRoute,
  loginRedirectRoute,
  setupRoute,
  shellRoute.addChildren([
    indexRoute,
    overviewRoute,
    projectsRoute,
    projectDetailRoute,
    conversationsRoute,
    sessionDetailRoute,
    eventDetailRoute,
    automationsRoute,
    assetsRoute,
    artifactDetailRoute,
    agentsRoute,
    skillDetailRoute,
    reportsRoute,
    auditRoute,
    accountRoute,
    settingsRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
