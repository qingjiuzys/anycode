import React, { lazy, Suspense, type ComponentType, type ReactNode } from "react";

const CHUNK_RELOAD_KEY = "anycode-dashboard-chunk-reload";

function isChunkLoadError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return (
    message.includes("Failed to fetch dynamically imported module") ||
    message.includes("Importing a module script failed") ||
    message.includes("Loading chunk")
  );
}

function reloadForFreshChunks() {
  const url = new URL(window.location.href);
  url.searchParams.set("__chunk_reload", Date.now().toString());
  window.location.replace(url.toString());
}

function lazyWithChunkReload<T extends { default: ComponentType<unknown> }>(
  importer: () => Promise<T>,
) {
  return lazy(async () => {
    try {
      const mod = await importer();
      sessionStorage.removeItem(CHUNK_RELOAD_KEY);
      return mod;
    } catch (error) {
      if (isChunkLoadError(error) && sessionStorage.getItem(CHUNK_RELOAD_KEY) !== "1") {
        sessionStorage.setItem(CHUNK_RELOAD_KEY, "1");
        reloadForFreshChunks();
      }
      throw error;
    }
  });
}

export const HomePage = lazyWithChunkReload(() =>
  import("@/pages/HomePage").then((m) => ({ default: m.HomePage })),
);
export const ProjectsPage = lazyWithChunkReload(() =>
  import("@/pages/ProjectsPage").then((m) => ({ default: m.ProjectsPage })),
);
export const ProjectDetailPage = lazyWithChunkReload(() =>
  import("@/pages/ProjectDetailPage").then((m) => ({ default: m.ProjectDetailPage })),
);
export const ConversationsPage = lazyWithChunkReload(() =>
  import("@/pages/ConversationsPage").then((m) => ({ default: m.ConversationsPage })),
);
export const SessionDetailPage = lazyWithChunkReload(() =>
  import("@/pages/SessionDetailPage").then((m) => ({ default: m.SessionDetailPage })),
);
export const EventDetailPage = lazyWithChunkReload(() =>
  import("@/pages/EventDetailPage").then((m) => ({ default: m.EventDetailPage })),
);
export const AutomationsPage = lazyWithChunkReload(() =>
  import("@/pages/AutomationsPage").then((m) => ({ default: m.AutomationsPage })),
);
export const AssetsPage = lazyWithChunkReload(() =>
  import("@/pages/AssetsPage").then((m) => ({ default: m.AssetsPage })),
);
export const ArtifactDetailPage = lazyWithChunkReload(() =>
  import("@/pages/ArtifactDetailPage").then((m) => ({ default: m.ArtifactDetailPage })),
);
export const AgentsPage = lazyWithChunkReload(() =>
  import("@/pages/AgentsPage").then((m) => ({ default: m.AgentsPage })),
);
export const SkillDetailPage = lazyWithChunkReload(() =>
  import("@/pages/SkillDetailPage").then((m) => ({ default: m.SkillDetailPage })),
);
export const ReportsPage = lazyWithChunkReload(() =>
  import("@/pages/ReportsPage").then((m) => ({ default: m.ReportsPage })),
);
export const AuditPage = lazyWithChunkReload(() =>
  import("@/pages/AuditPage").then((m) => ({ default: m.AuditPage })),
);
export const SettingsPage = lazyWithChunkReload(() =>
  import("@/pages/SettingsPage").then((m) => ({ default: m.SettingsPage })),
);
export const LoginPage = lazyWithChunkReload(() =>
  import("@/pages/LoginPage").then((m) => ({ default: m.LoginPage })),
);

function PageLoadError({ error }: { error: unknown }) {
  const message = error instanceof Error ? error.message : String(error);
  const staleChunk = isChunkLoadError(error);
  return (
    <div className="dw-alert-error max-w-2xl">
      <p className="font-medium m-0">
        {staleChunk ? "Dashboard UI was updated" : "This page failed to load"}
      </p>
      <p className="text-sm mt-1 m-0 break-words">
        {staleChunk
          ? "Your browser still has an old bundle. Reload the page to fetch the latest UI."
          : message}
      </p>
      <button
        type="button"
        className="dw-btn-primary mt-3"
        onClick={() => {
          sessionStorage.removeItem(CHUNK_RELOAD_KEY);
          window.location.reload();
        }}
      >
        Reload page
      </button>
    </div>
  );
}

class PageErrorBoundary extends React.Component<
  { children: ReactNode },
  { error: unknown | null }
> {
  state = { error: null as unknown | null };

  static getDerivedStateFromError(error: unknown) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return <PageLoadError error={this.state.error} />;
    }
    return this.props.children;
  }
}

export function Page({ children }: { children: ReactNode }) {
  return (
    <PageErrorBoundary>
      <Suspense fallback={<div className="p-6 text-secondary text-sm">Loading…</div>}>
        {children}
      </Suspense>
    </PageErrorBoundary>
  );
}
