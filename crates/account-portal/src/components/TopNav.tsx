import { Link, useLocation } from "react-router-dom";
import { LocaleSwitcher } from "./LocaleSwitcher";
import { Logo } from "./Logo";
import { useAuth } from "../hooks/useAuth";
import { useHomeHeaderScroll } from "../hooks/useHomeHeaderScroll";
import { SITE_PATHS } from "@anycode/site-urls";
import { useT } from "../i18n/context";

/** Unified marketing header — same controls on home / features / docs / legal. */
export function TopNav() {
  const loc = useLocation();
  const { authenticated, logout } = useAuth();
  const t = useT();

  const isHome = loc.pathname === "/" || loc.pathname === "/home-classic";
  const isFeatures = loc.pathname === SITE_PATHS.features;
  const isProduct = loc.pathname === SITE_PATHS.product;
  const isPlans = loc.pathname === SITE_PATHS.plans;
  const isDownloads = loc.pathname === SITE_PATHS.downloads;
  const isDocs = loc.pathname.startsWith(SITE_PATHS.docs);
  const isConsole = loc.pathname.startsWith("/console");
  const scrolled = useHomeHeaderScroll(true);

  const headerClass = ["lx-header", scrolled || !isHome ? "is-scrolled" : ""]
    .filter(Boolean)
    .join(" ");

  return (
    <header className={headerClass} data-theme="dark">
      <div className="lx-header__inner">
        <Link className="lx-header__brand" to="/">
          <Logo size="sm" />
          <span>{t("common.brand")}</span>
        </Link>

        <nav className="lx-header__nav" aria-label={t("nav.aria")}>
          <Link className={`lx-header__link${isHome ? " active" : ""}`} to="/">
            {t("nav.home")}
          </Link>
          <Link className={`lx-header__link${isFeatures ? " active" : ""}`} to={SITE_PATHS.features}>
            {t("nav.features")}
          </Link>
          <Link className={`lx-header__link${isProduct ? " active" : ""}`} to={SITE_PATHS.product}>
            {t("nav.product")}
          </Link>
          <Link className={`lx-header__link${isPlans ? " active" : ""}`} to={SITE_PATHS.plans}>
            {t("nav.plans")}
          </Link>
          <Link
            className={`lx-header__link${isDownloads ? " active" : ""}`}
            to={SITE_PATHS.downloads}
          >
            {t("nav.downloads")}
          </Link>
          <Link className={`lx-header__link${isDocs ? " active" : ""}`} to={SITE_PATHS.docs}>
            {t("nav.docs")}
          </Link>
        </nav>

        <div className="lx-header__actions">
          <LocaleSwitcher variant="header" />
          {authenticated ? (
            <>
              <Link className="lx-btn lx-btn--ghost" to="/console">
                {t("nav.console")}
              </Link>
              {!isConsole ? (
                <button
                  type="button"
                  className="lx-btn lx-btn--ghost"
                  onClick={() => {
                    logout();
                    window.location.href = "/";
                  }}
                >
                  {t("nav.signOut")}
                </button>
              ) : null}
            </>
          ) : (
            <>
              <Link className="lx-btn lx-btn--ghost" to="/login">
                {t("nav.signIn")}
              </Link>
              <Link className="lx-btn lx-btn--primary" to="/register">
                {t("nav.getStarted")}
              </Link>
            </>
          )}
        </div>
      </div>
    </header>
  );
}
