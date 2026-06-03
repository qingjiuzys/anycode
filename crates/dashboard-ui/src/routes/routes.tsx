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
  ): {
    status?: string;
    trusted?: string;
    kind?: string;
    needs_approval?: boolean;
    budget_exceeded?: boolean;
    project?: string;
    session?: string;
    agent?: string;
    filter?: "all" | "running" | "blocked" | "workflow" | "cron" | "needs_approval" | "budget";
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
    const status =
      typeof search.status === "string" && search.status.trim()
        ? search.status.trim()
        : undefined;
    const trusted =
      typeof search.trusted === "string" && search.trusted.trim()
        ? search.trusted.trim()
        : undefined;
    const kind =
      typeof search.kind === "string" && search.kind.trim()
        ? search.kind.trim()
        : undefined;
    const needs_approval =
      search.needs_approval === true ||
      search.needs_approval === "true" ||
      search.needs_approval === "1";
    const budget_exceeded =
      search.budget_exceeded === true ||
      search.budget_exceeded === "true" ||
      search.budget_exceeded === "1";
    const f = search.filter;
    const legacyFilter =
      f === "running" ||
      f === "blocked" ||
      f === "workflow" ||
      f === "cron" ||
      f === "needs_approval" ||
      f === "budget" ||
      f === "all"
        ? f
        : undefined;
    return {
      status,
      trusted,
      kind,
      needs_approval: needs_approval || undefined,
      budget_exceeded: budget_exceeded || undefined,
      project,
      session,
      agent,
      filter: legacyFilter,
    };
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
  validateSearch: (
    search: Record<string, unknown>,
  ): { section?: SettingsSection } => {
    const section = search.section;
    const valid = [
      "auth",
      "data",
      "service",
      "model",
      "skills",
      "assets",
      "security",
      "notify",
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
