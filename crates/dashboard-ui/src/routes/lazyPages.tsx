import { lazy, Suspense, type ReactNode } from "react";

export const HomePage = lazy(() =>
  import("@/pages/HomePage").then((m) => ({ default: m.HomePage })),
);
export const ProjectsPage = lazy(() =>
  import("@/pages/ProjectsPage").then((m) => ({ default: m.ProjectsPage })),
);
export const ProjectDetailPage = lazy(() =>
  import("@/pages/ProjectDetailPage").then((m) => ({ default: m.ProjectDetailPage })),
);
export const ConversationsPage = lazy(() =>
  import("@/pages/ConversationsPage").then((m) => ({ default: m.ConversationsPage })),
);
export const SessionDetailPage = lazy(() =>
  import("@/pages/SessionDetailPage").then((m) => ({ default: m.SessionDetailPage })),
);
export const EventDetailPage = lazy(() =>
  import("@/pages/EventDetailPage").then((m) => ({ default: m.EventDetailPage })),
);
export const AutomationsPage = lazy(() =>
  import("@/pages/AutomationsPage").then((m) => ({ default: m.AutomationsPage })),
);
export const AssetsPage = lazy(() =>
  import("@/pages/AssetsPage").then((m) => ({ default: m.AssetsPage })),
);
export const ArtifactDetailPage = lazy(() =>
  import("@/pages/ArtifactDetailPage").then((m) => ({ default: m.ArtifactDetailPage })),
);
export const AgentsPage = lazy(() =>
  import("@/pages/AgentsPage").then((m) => ({ default: m.AgentsPage })),
);
export const SkillDetailPage = lazy(() =>
  import("@/pages/SkillDetailPage").then((m) => ({ default: m.SkillDetailPage })),
);
export const ReportsPage = lazy(() =>
  import("@/pages/ReportsPage").then((m) => ({ default: m.ReportsPage })),
);
export const AuditPage = lazy(() =>
  import("@/pages/AuditPage").then((m) => ({ default: m.AuditPage })),
);
export const SettingsPage = lazy(() =>
  import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })),
);
export const LoginPage = lazy(() =>
  import("@/pages/LoginPage").then((m) => ({ default: m.LoginPage })),
);

export function Page({ children }: { children: ReactNode }) {
  return <Suspense fallback={<div className="p-6 text-secondary text-sm">…</div>}>{children}</Suspense>;
}
