import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { Layout } from "@/components/Layout";
import { api } from "@/api/client";
import {
  AgentsPage,
  ArtifactDetailPage,
  AssetsPage,
  AuditPage,
  AutomationsPage,
  ConversationsPage,
  EventDetailPage,
  HomePage,
  LoginPage,
  Page,
  ProjectDetailPage,
  ProjectsPage,
  ReportsPage,
  SessionDetailPage,
  SettingsPage,
  SkillDetailPage,
} from "@/routes/lazyPages";

export const rootRoute = createRootRoute({
  component: () => <Outlet />,
});

export const shellRoute = createRoute({
  id: "_shell",
  getParentRoute: () => rootRoute,
  component: Layout,
  beforeLoad: async () => {
    try {
      const svc = await api.serviceStatus();
      if (svc.service.loopback) return;
    } catch {
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

export const indexRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  component: () => (
    <Page>
      <HomePage />
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
  validateSearch: (
    search: Record<string, unknown>,
  ): { filter?: "all" | "running" | "blocked" | "workflow" | "cron" | "needs_approval" } => {
    const f = search.filter;
    if (
      f === "running" ||
      f === "blocked" ||
      f === "workflow" ||
      f === "cron" ||
      f === "needs_approval" ||
      f === "all"
    ) {
      return { filter: f };
    }
    return {};
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
  component: () => (
    <Page>
      <SettingsPage />
    </Page>
  ),
});

export const routeTree = rootRoute.addChildren([
  loginRoute,
  shellRoute.addChildren([
    indexRoute,
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
