import { useEffect, useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { useAuth } from "@/auth/context";
import { useI18n } from "@/i18n/context";

export function UserMenu() {
  const { user, authenticated, logout } = useAuth();
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  if (!authenticated || !user) {
    return (
      <Link to="/login" className="dw-btn-primary no-underline">
        <Icon name="login" size={16} />
        {t("auth.signIn")}
      </Link>
    );
  }

  const initials = user.display_name
    .split(/\s+/)
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        className="flex items-center gap-2 rounded-full border border-outline-variant bg-surface-container-lowest pl-1 pr-2 py-1 hover:bg-surface-container transition-colors"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        <span className="w-8 h-8 rounded-full bg-primary text-on-primary text-xs font-semibold flex items-center justify-center overflow-hidden">
          {initials || <Icon name="account_circle" size={22} className="text-on-primary" />}
        </span>
        <span className="text-sm font-medium text-on-surface max-w-[120px] truncate hidden md:inline">
          {user.display_name}
        </span>
        <Icon name="expand_more" size={18} className="text-secondary hidden sm:inline" />
      </button>
      {open && (
        <div
          className="absolute right-0 top-full mt-2 w-72 bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg z-50 py-2"
          role="menu"
        >
          <div className="px-4 py-3 border-b border-outline-variant">
            <div className="flex items-center gap-3">
              <span className="w-10 h-10 rounded-full bg-primary text-on-primary text-sm font-semibold flex items-center justify-center">
                {initials || "U"}
              </span>
              <div className="min-w-0">
                <p className="text-sm font-semibold m-0 truncate">{user.display_name}</p>
                <p className="text-xs text-secondary m-0 mt-0.5 font-code truncate">{user.email}</p>
              </div>
            </div>
            <p className="text-xs text-secondary m-0 mt-2">
              {t("auth.role")}: {user.role} · {user.auth_method}
            </p>
          </div>
          <Link
            to="/account"
            search={{ section: "plan" }}
            className="flex items-center gap-2 px-4 py-2.5 text-sm no-underline hover:bg-surface-container text-on-surface"
            onClick={() => setOpen(false)}
            role="menuitem"
          >
            <Icon name="corporate_fare" size={18} />
            {t("service.menu.plans")}
          </Link>
          <Link
            to="/account"
            search={{ section: "usage" }}
            className="flex items-center gap-2 px-4 py-2.5 text-sm no-underline hover:bg-surface-container text-on-surface"
            onClick={() => setOpen(false)}
            role="menuitem"
          >
            <Icon name="analytics" size={18} />
            {t("service.menu.usage")}
          </Link>
          <Link
            to="/settings"
            className="flex items-center gap-2 px-4 py-2.5 text-sm no-underline hover:bg-surface-container text-on-surface"
            onClick={() => setOpen(false)}
            role="menuitem"
          >
            <Icon name="settings" size={18} />
            {t("nav.settings")}
          </Link>
          <button
            type="button"
            className="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-left hover:bg-surface-container text-error border-0 bg-transparent cursor-pointer"
            onClick={() => {
              setOpen(false);
              void logout();
            }}
            role="menuitem"
          >
            <Icon name="logout" size={18} />
            {t("auth.signOut")}
          </button>
        </div>
      )}
    </div>
  );
}

export function LanguageSwitcher() {
  const { locale, setLocale, t } = useI18n();
  return (
    <div
      className="flex items-center rounded-lg border border-outline-variant overflow-hidden text-xs"
      role="group"
      aria-label={t("common.language")}
    >
      <button
        type="button"
        className={`px-2.5 py-1.5 border-0 cursor-pointer ${locale === "zh" ? "bg-surface-container-high text-primary font-medium" : "bg-surface text-secondary hover:bg-surface-container"}`}
        onClick={() => setLocale("zh")}
      >
        {t("common.zh")}
      </button>
      <button
        type="button"
        className={`px-2.5 py-1.5 border-0 border-l border-outline-variant cursor-pointer ${locale === "en" ? "bg-surface-container-high text-primary font-medium" : "bg-surface text-secondary hover:bg-surface-container"}`}
        onClick={() => setLocale("en")}
      >
        {t("common.en")}
      </button>
    </div>
  );
}
