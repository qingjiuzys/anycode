import { useState } from "react";
import { Outlet, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { TopbarSearch } from "@/components/TopbarSearch";
import { TopbarNewMenu } from "@/components/TopbarNewMenu";
import { AppearanceMenu } from "@/components/AppearanceMenu";
import { Icon } from "@/components/Icon";
import { SseStatusBadge } from "@/components/SseStatusBadge";
import { UserMenu, LanguageSwitcher } from "@/components/UserMenu";
import { NotificationsDropdown } from "@/components/NotificationsDropdown";
import { ControlCenterButton } from "@/components/control-center/ControlCenterButton";
import { ControlCenterOverlay } from "@/components/control-center/ControlCenterOverlay";
import { SessionSidebar } from "@/components/session/SessionSidebar";
import { useAuth } from "@/auth/context";
import { useI18n } from "@/i18n/context";
import { docsHomeUrl, helpGuideUrl } from "@/lib/docLinks";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useSseStatus } from "@/context/SseContext";
import { FeatureRouteSync } from "@/components/control-center/FeatureRouteSync";
import { ControlCenterProvider } from "@/context/ControlCenterContext";
import { ConversationShellProvider } from "@/context/ConversationShellContext";
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

function Topbar({ compact = false }: { compact?: boolean }) {
  const { t, locale } = useI18n();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const sseStatus = useSseStatus();

  return (
    <header className="dw-topbar glass-panel">
      <div className={`dw-topbar-start ${pathname === "/" ? "dw-topbar-start--empty" : ""}`}>
        {!compact && pathname !== "/" && (
          <div className="dw-topbar-hit w-full min-w-0">
            <TopbarSearch />
          </div>
        )}
      </div>
      <div className="dw-topbar-end">
        {!compact && pathname !== "/" && (
          <div className="hidden xl:block dw-topbar-hit">
            <SseStatusBadge status={mapSseStatus(sseStatus)} />
          </div>
        )}
        <div className="dw-topbar-hit">
          <LanguageSwitcher />
        </div>
        <div className="w-px h-6 bg-outline-variant hidden sm:block shrink-0" />
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
          <TopbarNewMenu />
        </div>
        <div className="dw-topbar-hit">
          <UserMenu />
        </div>
      </div>
    </header>
  );
}

function SessionFirstShell() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  return (
    <ConversationShellProvider>
      <div
        className={`dw-shell dw-shell--sessions${sidebarCollapsed ? " dw-shell--sessions-collapsed" : ""}`}
      >
        <SessionSidebar
          collapsed={sidebarCollapsed}
          onCollapsedChange={setSidebarCollapsed}
        />
        <div
          className={`dw-main-wrap dw-main-wrap--sessions${sidebarCollapsed ? " dw-main-wrap--sessions-collapsed" : ""}`}
        >
          <Topbar compact />
          <main className="dw-main dw-main--sessions">
            <Outlet />
          </main>
        </div>
        <ControlCenterButton />
        <ControlCenterOverlay />
      </div>
    </ConversationShellProvider>
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
      <ControlCenterButton />
      <ControlCenterOverlay />
      <footer className="dw-standard-footer hidden lg:flex">
        <AppearanceMenu />
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
