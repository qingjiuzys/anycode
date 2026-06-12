import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { Layout } from "@/components/Layout";
import type { SettingsSection } from "@/components/settings/SettingsNav";
import { api } from "@/api/client";
import {
  conversationSearchParams,
  conversationsCanonicalHref,
  parseConversationSearch,
} from "@/lib/conversationsSearch";
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
  LoginPage,
  Page,
  ProjectDetailPage,
  ProjectsPage,
  ReportsPage,
  SessionDetailPage,
  SettingsPage,
  SetupWizardPage,
  SkillDetailPage,
} from "@/routes/lazyPages";

export const rootRoute = createRootRoute({
  component: () => <Outlet />,
});

export const shellRoute = createRoute({
  id: "_shell",
  getParentRoute: () => rootRoute,
  component: Layout,
  beforeLoad: async ({ location }) => {
    try {
      const svc = await api.serviceStatus();
      if (svc.service.loopback) {
        const setup = await api.setupStatus();
        const review = new URLSearchParams(location.search).get("review") === "1";
        if (!setup.setup.ready && !review && location.pathname !== "/setup") {
          throw redirect({
            to: "/setup",
            search: { review: undefined, step: undefined, tab: undefined },
          });
        }
        return;
      }
    } catch (e) {
      if (e && typeof e === "object" && "to" in e) throw e;
      return;
    }
    const me = await api.authMe();
    if (!me.authenticated) {
      throw redirect({ to: "/login" });
    }
  },
});

export const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  component: () => (
    <Page>
      <LoginPage />
    </Page>
  ),
});

export const setupRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/setup",
  validateSearch: (search: Record<string, unknown>) => ({
    review: typeof search.review === "string" ? search.review : undefined,
    step: typeof search.step === "string" ? search.step : undefined,
    tab: typeof search.tab === "string" ? search.tab : undefined,
  }),
  component: () => (
    <Page>
      <SetupWizardPage />
    </Page>
  ),
});

export const indexRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
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
    const base = { project, session, agent };

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

export const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/settings",
  validateSearch: (
    search: Record<string, unknown>,
  ): { section?: SettingsSection } => {
    const section = search.section;
    const valid = [
      "auth",
      "prefs",
      "data",
      "service",
      "model",
      "agents",
      "skills",
      "assets",
      "security",
      "notify",
      "channels",
      "ops",
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
  loginRoute,
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
    settingsRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
