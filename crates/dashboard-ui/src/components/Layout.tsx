import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { TopbarSearch } from "@/components/TopbarSearch";
import { TopbarNewMenu } from "@/components/TopbarNewMenu";
import { SidebarWorkspaceCard } from "@/components/SidebarWorkspaceCard";
import { AppearanceMenu } from "@/components/AppearanceMenu";
import { Icon } from "@/components/Icon";
import { SseStatusBadge } from "@/components/SseStatusBadge";
import { UserMenu, LanguageSwitcher } from "@/components/UserMenu";
import { NotificationsDropdown } from "@/components/NotificationsDropdown";
import { useAuth } from "@/auth/context";
import { useI18n } from "@/i18n/context";
import { docsHomeUrl, helpGuideUrl } from "@/lib/docLinks";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useSseStatus } from "@/context/SseContext";
import { api } from "@/api/client";
import { BrandMark } from "@/components/BrandMark";

const NAV = [
  { to: "/", key: "nav.home", icon: "home", countKey: null },
  { to: "/overview", key: "nav.overview", icon: "dashboard", countKey: null },
  { to: "/projects", key: "nav.projects", icon: "folder", countKey: "projects" as const },
  { to: "/conversations", key: "nav.conversations", icon: "chat", countKey: "sessions" as const },
  { to: "/automations", key: "nav.automations", icon: "settings_suggest", countKey: null },
  { to: "/assets", key: "nav.assets", icon: "inventory_2", countKey: "artifacts" as const },
  { to: "/reports", key: "nav.reports", icon: "bar_chart", countKey: null },
  { to: "/audit", key: "nav.audit", icon: "verified_user", countKey: null },
  { to: "/agents", key: "nav.agents", icon: "robot_2", countKey: "skills" as const },
  { to: "/settings", key: "nav.settings", icon: "settings", countKey: null },
] as const;

function mapSseStatus(status: string): "connecting" | "live" | "reconnecting" | "offline" {
  if (status === "live") return "live";
  if (status === "connecting") return "connecting";
  if (status === "reconnecting") return "reconnecting";
  return "offline";
}

function navCount(
  key: "projects" | "sessions" | "artifacts" | "skills" | null,
  ov?: { projects_count: number; sessions_total: number; artifacts_count: number; skills_count: number },
): number | null {
  if (!key || !ov) return null;
  switch (key) {
    case "projects":
      return ov.projects_count;
    case "sessions":
      return ov.sessions_total;
    case "artifacts":
      return ov.artifacts_count;
    case "skills":
      return ov.skills_count;
    default:
      return null;
  }
}

export function Layout() {
  const { t, locale } = useI18n();
  const { loading: authLoading } = useAuth();
  const sseStatus = useSseStatus();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview });

  const isActive = (to: string) =>
    to === "/" ? pathname === "/" : pathname === to || pathname.startsWith(`${to}/`);

  if (authLoading) {
    return (
      <div className="h-full flex items-center justify-center text-secondary">
        {t("common.loading")}
      </div>
    );
  }

  const ov = overview.data?.overview;

  return (
    <div className="dw-shell">
      <aside className="dw-sidebar glass-panel">
        <div className="dw-sidebar-brand">
          <BrandMark size="md" showTitle linked />
        </div>

        <SidebarWorkspaceCard />

        <nav className="flex-1 min-h-0 overflow-y-auto overscroll-y-contain px-2 space-y-0.5">
          {NAV.map((item) => {
            const count = navCount(item.countKey, ov);
            return (
              <Link
                key={item.to}
                to={item.to}
                className={`dw-nav-link ${isActive(item.to) ? "active" : ""}`}
              >
                <Icon name={item.icon} filled={isActive(item.to)} size={18} />
                <span className="flex-1 min-w-0 truncate">{t(item.key)}</span>
                {count != null && count > 0 && (
                  <span className="text-[10px] font-semibold tabular-nums px-1.5 py-0.5 rounded-full bg-surface-container-high text-secondary">
                    {count}
                  </span>
                )}
              </Link>
            );
          })}
        </nav>

        <div className="dw-sidebar-footer">
          <AppearanceMenu />
          <ExternalNavLink href={docsHomeUrl(locale)} className="dw-nav-link">
            <Icon name="description" size={18} />
            <span className="flex-1 min-w-0 truncate">{t("nav.docs")}</span>
          </ExternalNavLink>
          <ExternalNavLink href={helpGuideUrl(locale)} className="dw-nav-link">
            <Icon name="help_outline" size={18} />
            <span className="flex-1 min-w-0 truncate">{t("nav.help")}</span>
          </ExternalNavLink>
          <div className="dw-sidebar-version" aria-label={t("layout.version")}>
            <span className="dw-sidebar-version-gutter" aria-hidden />
            <span className="tabular-nums">v{health.data?.version ?? "…"}</span>
          </div>
        </div>
      </aside>

      <div className="dw-main-wrap">
        <header className="dw-topbar glass-panel">
          <div className={`dw-topbar-start ${pathname === "/" ? "dw-topbar-start--empty" : ""}`}>
            {pathname !== "/" && (
              <div className="dw-topbar-hit w-full min-w-0">
                <TopbarSearch />
              </div>
            )}
          </div>
          <div className="dw-topbar-end">
            {pathname !== "/" && (
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
        <main className="dw-main">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
