import { Outlet, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { TopbarSearch } from "@/components/TopbarSearch";
import { Icon } from "@/components/Icon";
import { SseStatusBadge } from "@/components/SseStatusBadge";
import { UserMenu } from "@/components/UserMenu";
import { NotificationsDropdown } from "@/components/NotificationsDropdown";
import { ControlCenterOverlay } from "@/components/control-center/ControlCenterOverlay";
import { SessionSidebar } from "@/components/session/SessionSidebar";
import { useAuth } from "@/auth/context";
import { useI18n } from "@/i18n/context";
import { docsHomeUrl, helpGuideUrl } from "@/lib/docLinks";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useSseStatus } from "@/context/SseContext";
import { FeatureRouteSync } from "@/components/control-center/FeatureRouteSync";
import { ControlCenterProvider } from "@/context/ControlCenterContext";
import { ConversationShellProvider, useConversationShell } from "@/context/ConversationShellContext";
import { api } from "@/api/client";

function isFullPageShellRoute(pathname: string, searchStr: string): boolean {
  if (pathname.startsWith("/events/")) return true;
  if (!pathname.startsWith("/sessions/")) return false;
  const tab = new URLSearchParams(searchStr.startsWith("?") ? searchStr.slice(1) : searchStr).get(
    "tab",
  );
  return tab === "debug" || tab === "audit";
}

function mapSseStatus(status: string): "connecting" | "live" | "reconnecting" | "offline" {
  if (status === "live") return "live";
  if (status === "connecting") return "connecting";
  if (status === "reconnecting") return "reconnecting";
  return "offline";
}

function Topbar({ compact = false, hideProfile = false }: { compact?: boolean; hideProfile?: boolean }) {
  const { t, locale } = useI18n();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const sseStatus = useSseStatus();
  const showSse = !compact && pathname !== "/";
  const showProfileCluster = !hideProfile;
  const showEnd = showSse || showProfileCluster;

  return (
    <header className={`dw-topbar glass-panel${pathname === "/" ? " dw-topbar--home" : ""}`}>
      {pathname !== "/" ? (
        <div className="dw-topbar-start">
          {!compact && (
            <div className="dw-topbar-hit w-full min-w-0">
              <TopbarSearch />
            </div>
          )}
        </div>
      ) : null}
      {showEnd ? (
        <div className="dw-topbar-end">
          {showSse ? (
            <div className="hidden xl:block dw-topbar-hit">
              <SseStatusBadge status={mapSseStatus(sseStatus)} />
            </div>
          ) : null}
          {showProfileCluster ? (
            <>
              <div className="dw-topbar-hit">
                <NotificationsDropdown />
              </div>
              <ExternalNavLink
                href={helpGuideUrl(locale)}
                className="dw-btn-secondary hidden md:inline-flex no-underline dw-topbar-hit"
              >
                {t("nav.help")}
              </ExternalNavLink>
              <div className="dw-topbar-hit">
                <UserMenu />
              </div>
            </>
          ) : null}
        </div>
      ) : null}
    </header>
  );
}

function SessionFirstShell() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const isHome = pathname === "/";

  return (
    <ConversationShellProvider>
      <SessionFirstShellInner isHome={isHome} />
    </ConversationShellProvider>
  );
}

function SessionFirstShellInner({ isHome }: { isHome: boolean }) {
  const { t } = useI18n();
  const { sessionSidebarCollapsed, setSessionSidebarCollapsed } = useConversationShell();

  return (
    <div
      className={`dw-shell dw-shell--sessions${
        sessionSidebarCollapsed ? " dw-sessions-sidebar-collapsed" : ""
      }`}
      data-tauri-drag-region
    >
      <SessionSidebar />
      <div className="dw-main-wrap dw-main-wrap--sessions">
        {/* No shell Topbar: home has no chrome strip; conversations use the thread title bar. */}
        <main className={`dw-main dw-main--sessions${isHome ? " dw-main--home" : ""}`}>
          {sessionSidebarCollapsed && isHome ? (
            <button
              type="button"
              className="dw-sessions-expand-fab"
              aria-label={t("conversations.expandSessions")}
              title={t("conversations.expandSessions")}
              onClick={() => setSessionSidebarCollapsed(false)}
            >
              <Icon name="view_sidebar" size={18} className="scale-x-[-1]" />
            </button>
          ) : null}
          <Outlet />
        </main>
      </div>
      <ControlCenterOverlay />
    </div>
  );
}

function StandardShell() {
  const { t, locale } = useI18n();
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });

  return (
    <div className="dw-shell dw-shell--standard">
      <div className="dw-main-wrap dw-main-wrap--full">
        <Topbar />
        <main className="dw-main">
          <Outlet />
        </main>
      </div>
      <ControlCenterOverlay />
      <footer className="dw-standard-footer hidden lg:flex">
        <ExternalNavLink href={docsHomeUrl(locale)} className="dw-nav-link">
          <Icon name="description" size={18} />
          <span>{t("nav.docs")}</span>
        </ExternalNavLink>
        <span className="text-[10px] text-secondary tabular-nums ml-auto">
          v{health.data?.version ?? "…"}
        </span>
      </footer>
    </div>
  );
}

export function Layout() {
  const { t } = useI18n();
  const { loading: authLoading } = useAuth();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const searchStr = useRouterState({ select: (s) => s.location.searchStr });
  const isFullPageRoute = isFullPageShellRoute(pathname, searchStr);

  if (authLoading) {
    return (
      <div className="h-full flex items-center justify-center text-secondary">
        {t("common.loading")}
      </div>
    );
  }

  return (
    <ControlCenterProvider>
      <FeatureRouteSync />
      {isFullPageRoute ? <StandardShell /> : <SessionFirstShell />}
    </ControlCenterProvider>
  );
}
