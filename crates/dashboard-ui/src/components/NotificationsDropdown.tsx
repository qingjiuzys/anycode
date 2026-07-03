import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { Link } from "@tanstack/react-router";
import { buildConversationsHref } from "@/lib/conversationsSearch";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

const MENU_WIDTH = 320;

function useCompactMenuStyle(open: boolean, anchorRef: React.RefObject<HTMLElement | null>) {
  const [style, setStyle] = useState<CSSProperties>({});

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) return;
    const update = () => {
      const rect = anchorRef.current!.getBoundingClientRect();
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - MENU_WIDTH - 8));
      setStyle({
        position: "fixed",
        left,
        bottom: window.innerHeight - rect.top + 8,
        width: MENU_WIDTH,
        zIndex: 300,
      });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open, anchorRef]);

  return style;
}

function NotificationMenu({
  className,
  style,
  menuRef,
  blockedCount,
  feedLoading,
  items,
  onClose,
  t,
}: {
  className: string;
  style?: CSSProperties;
  menuRef?: React.RefObject<HTMLDivElement | null>;
  blockedCount: number;
  feedLoading: boolean;
  items: Array<{ id: string; title?: string | null; action: string; detail?: string | null; created_at: string }>;
  onClose: () => void;
  t: (key: string) => string;
}) {
  return (
    <div ref={menuRef} role="menu" className={className} style={style}>
      <div className="px-4 py-2 border-b border-outline-variant flex items-center justify-between">
        <span className="text-sm font-semibold">{t("notifications.title")}</span>
        <Link
          to="/audit"
          className="text-xs text-primary no-underline hover:underline"
          onClick={onClose}
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
            to={buildConversationsHref({ filter: "blocked" })}
            className="text-sm text-error no-underline hover:underline"
            onClick={onClose}
          >
            {t("home.insightBlocked").replace("{n}", String(blockedCount))}
          </Link>
          <p className="text-[10px] text-secondary m-0 mt-1">{t("notifications.blockedHint")}</p>
        </div>
      )}
      {feedLoading && (
        <p className="px-4 py-3 text-sm text-secondary m-0">{t("common.loading")}</p>
      )}
      {!feedLoading && items.length === 0 && blockedCount === 0 && (
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
  );
}

export function NotificationsDropdown({ compact = false }: { compact?: boolean }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const compactMenuStyle = useCompactMenuStyle(open && compact, ref);

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
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const target = e.target as Node;
      if (ref.current?.contains(target)) return;
      if (menuRef.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  function toggleOpen() {
    setOpen((v) => !v);
  }

  const menuProps = {
    blockedCount,
    feedLoading: feed.isLoading,
    items,
    onClose: () => setOpen(false),
    t,
  };

  const menuClass =
    "bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg py-2";

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        className={compact ? "dw-session-sidebar-footer__icon-btn relative" : "dw-btn-ghost p-2 relative"}
        title={t("layout.notifications")}
        aria-label={t("layout.notifications")}
        aria-expanded={open}
        aria-haspopup="menu"
        onPointerDown={(e) => {
          e.stopPropagation();
        }}
        onClick={(e) => {
          e.stopPropagation();
          toggleOpen();
        }}
      >
        <Icon name="notifications" size={20} />
        {badgeCount > 0 && (
          <span className="absolute -top-0.5 -right-0.5 min-w-[1rem] h-4 px-1 rounded-full bg-error text-[10px] font-semibold text-on-error flex items-center justify-center">
            {badgeCount > 9 ? "9+" : badgeCount}
          </span>
        )}
      </button>
      {open &&
        (compact ? (
          createPortal(
            <NotificationMenu
              {...menuProps}
              menuRef={menuRef}
              className={menuClass}
              style={compactMenuStyle}
            />,
            document.body,
          )
        ) : (
          <NotificationMenu
            {...menuProps}
            className={`absolute right-0 top-full mt-2 w-80 z-[110] ${menuClass}`}
          />
        ))}
    </div>
  );
}
