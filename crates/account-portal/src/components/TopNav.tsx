import { Link, useLocation } from "react-router-dom";
import { LocaleSwitcher } from "./LocaleSwitcher";
import { Logo } from "./Logo";
import { useAuth } from "../hooks/useAuth";
import { useHomeHeaderScroll } from "../hooks/useHomeHeaderScroll";
import { SITE_GITHUB, SITE_PATHS } from "@anycode/site-urls";
import { useT } from "../i18n/context";

/** Unified marketing header — same controls on home / features / docs / legal. */
export function TopNav() {
  const loc = useLocation();
  const { authenticated, logout } = useAuth();
  const t = useT();

  const isConsole = loc.pathname.startsWith("/console");
  const isLegacyHome =
    loc.pathname === "/home-nx" || loc.pathname === "/home-classic";
  const isOrbitSite =
    !(import.meta.env.DEV && loc.pathname === "/__design-prototype") &&
    !isLegacyHome;
  const isHome =
    loc.pathname === "/" ||
    loc.pathname === "/home-nx" ||
    loc.pathname === "/home-classic";
  const isFeatures = loc.pathname === SITE_PATHS.features;
  const isProduct = loc.pathname === SITE_PATHS.product;
  const isPlans = loc.pathname === SITE_PATHS.plans;
  const isDownloads = loc.pathname === SITE_PATHS.downloads;
  const isChangelog = loc.pathname === SITE_PATHS.changelog;
  const isDocs = loc.pathname.startsWith(SITE_PATHS.docs);
  const scrolled = useHomeHeaderScroll(true);

  const headerClass = ["lx-header", scrolled || isHome || isOrbitSite ? "is-scrolled" : ""]
    .filter(Boolean)
    .join(" ");

  return (
    <header className={headerClass} data-theme={isOrbitSite ? "dark" : isHome ? "light" : "dark"}>
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
          <Link
            className={`lx-header__link${isChangelog ? " active" : ""}`}
            to={SITE_PATHS.changelog}
          >
            {t("nav.changelog")}
          </Link>
          <Link className={`lx-header__link${isDocs ? " active" : ""}`} to={SITE_PATHS.docs}>
            {t("nav.docs")}
          </Link>
        </nav>

        <div className="lx-header__actions">
          <a
            className="lx-header__icon-link"
            href={SITE_GITHUB}
            target="_blank"
            rel="noreferrer"
            aria-label={t("nav.openSource")}
            title={t("nav.openSource")}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
              <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.009-.868-.014-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844a9.59 9.59 0 0 1 2.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.02 10.02 0 0 0 22 12.017C22 6.484 17.522 2 12 2Z" />
            </svg>
          </a>
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
