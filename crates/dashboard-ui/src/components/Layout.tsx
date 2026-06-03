import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { TopbarSearch } from "@/components/TopbarSearch";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { SidebarWorkspaceCard } from "@/components/SidebarWorkspaceCard";
import { Icon } from "@/components/Icon";
import { SseStatusBadge } from "@/components/SseStatusBadge";
import { ThemeToggle } from "@/components/ThemeToggle";
import { UserMenu, LanguageSwitcher } from "@/components/UserMenu";
import { NotificationsDropdown } from "@/components/NotificationsDropdown";
import { useAuth } from "@/auth/context";
import { useI18n } from "@/i18n/context";
import { useSseStatus } from "@/context/SseContext";
import { api } from "@/api/client";
import brandLogo from "@/assets/anycode-logo.png";

const NAV = [
  { to: "/", key: "nav.overview", icon: "dashboard", countKey: null },
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
  const { t } = useI18n();
  const { loading: authLoading } = useAuth();
  const [newProjectOpen, setNewProjectOpen] = useState(false);
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
      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />

      <aside className="dw-sidebar">
        <div className="px-4 mb-4">
          <div className="flex items-center gap-2">
            <img
              src={brandLogo}
              alt=""
              className="w-8 h-8 rounded shadow-sm object-cover bg-surface-container-lowest"
            />
            <div>
              <div className="text-base font-semibold text-primary">{t("layout.brand")}</div>
              <div className="text-xs text-secondary">v{health.data?.version ?? "…"}</div>
            </div>
          </div>
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

        <div className="px-2 pt-4 mt-auto border-t border-outline-variant mx-2 space-y-0.5">
          <a
            href="https://anycode.dev/guide/dashboard"
            target="_blank"
            rel="noreferrer"
            className="dw-nav-link"
          >
            <Icon name="description" size={18} />
            {t("nav.docs")}
          </a>
          <Link to="/settings" className="dw-nav-link">
            <Icon name="help_outline" size={18} />
            {t("nav.support")}
          </Link>
        </div>
      </aside>

      <div className="dw-main-wrap">
        <header className="dw-topbar shrink-0">
          <div className="flex flex-1 items-center gap-3 min-w-0">
            <TopbarSearch />
            <div className="dw-topbar-drag-spacer" data-tauri-drag-region aria-hidden />
          </div>
          <div className="flex items-center gap-2 sm:gap-3 shrink-0">
            {pathname === "/" && (
              <div id="dw-home-panels-slot" className="flex items-center shrink-0" />
            )}
            <div className="hidden xl:block mr-1">
              <SseStatusBadge status={mapSseStatus(sseStatus)} />
            </div>
            <LanguageSwitcher />
            <ThemeToggle />
            <div className="w-px h-6 bg-outline-variant hidden sm:block" />
            <NotificationsDropdown />
            <Link to="/settings" className="dw-btn-secondary hidden md:inline-flex no-underline">
              {t("common.support")}
            </Link>
            <button
              type="button"
              className="dw-btn-primary hidden sm:inline-flex"
              onClick={() => setNewProjectOpen(true)}
            >
              <Icon name="add" size={16} />
              {t("layout.newProject")}
            </button>
            <UserMenu />
          </div>
        </header>
        <main className="dw-main">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
