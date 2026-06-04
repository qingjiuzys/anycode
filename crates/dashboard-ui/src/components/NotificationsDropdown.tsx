import { useEffect, useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function NotificationsDropdown() {
  const t = useT();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const overview = useQuery({
    queryKey: ["overview"],
    queryFn: api.overview,
    staleTime: 60_000,
  });

  const feed = useQuery({
    queryKey: ["notifications-recent"],
    queryFn: () => api.notificationsRecent(20),
    enabled: open,
    refetchInterval: open ? 15_000 : false,
    staleTime: 10_000,
  });

  const items = feed.data?.notifications ?? [];
  const blockedCount = overview.data?.overview.sessions_blocked ?? 0;
  const badgeCount = items.length;

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        className="dw-btn-ghost p-2 relative"
        title={t("layout.notifications")}
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <Icon name="notifications" size={20} />
        {badgeCount > 0 && (
          <span className="absolute -top-0.5 -right-0.5 min-w-[1rem] h-4 px-1 rounded-full bg-error text-[10px] font-semibold text-on-error flex items-center justify-center">
            {badgeCount > 9 ? "9+" : badgeCount}
          </span>
        )}
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-2 w-80 bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg z-50 py-2">
          <div className="px-4 py-2 border-b border-outline-variant flex items-center justify-between">
            <span className="text-sm font-semibold">{t("notifications.title")}</span>
            <Link
              to="/audit"
              className="text-xs text-primary no-underline hover:underline"
              onClick={() => setOpen(false)}
            >
              {t("notifications.viewAudit")}
            </Link>
          </div>
          {blockedCount > 0 && (
            <div className="px-4 py-2 border-b border-outline-variant bg-error/5">
              <div className="text-[10px] uppercase tracking-wide text-secondary mb-1">
                {t("notifications.blockedSessions")}
              </div>
              <Link
                to="/conversations"
                search={{ trusted: "blocked" }}
                className="text-sm text-error no-underline hover:underline"
                onClick={() => setOpen(false)}
              >
                {t("home.insightBlocked").replace("{n}", String(blockedCount))}
              </Link>
              <p className="text-[10px] text-secondary m-0 mt-1">{t("notifications.blockedHint")}</p>
            </div>
          )}
          {feed.isLoading && (
            <p className="px-4 py-3 text-sm text-secondary m-0">{t("common.loading")}</p>
          )}
          {!feed.isLoading && items.length === 0 && blockedCount === 0 && (
            <p className="px-4 py-3 text-sm text-secondary m-0">{t("notifications.empty")}</p>
          )}
          <ul className="m-0 p-0 list-none max-h-64 overflow-y-auto">
            {items.slice(0, 10).map((n) => (
              <li
                key={n.id}
                className="px-4 py-2 text-sm hover:bg-surface-container border-b border-outline-variant last:border-0"
              >
                <div className="font-medium">{n.title || n.action}</div>
                {n.detail && (
                  <p className="text-xs text-secondary m-0 mt-0.5 line-clamp-2">{n.detail}</p>
                )}
                <time className="text-[10px] text-outline">{n.created_at}</time>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
