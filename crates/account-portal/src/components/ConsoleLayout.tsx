import { NavLink, Outlet } from "react-router-dom";
import { Logo } from "./Logo";
import { useT } from "../i18n/context";

const MENU = [
  { to: "/console", labelKey: "console.overview", end: true, code: "01" },
  { to: "/console/usage", labelKey: "console.usage", code: "02" },
  { to: "/console/plans", labelKey: "console.plans", code: "03" },
  { to: "/console/billing", labelKey: "console.billing", code: "04" },
  { to: "/console/api", labelKey: "console.api", code: "05" },
  { to: "/console/settings", labelKey: "console.settings", code: "06" },
] as const;

export function ConsoleLayout() {
  const t = useT();

  return (
    <div className="nx-site nx-site--console">
      <div className="nx-frame nx-site__console">
        <aside className="nx-console-sidebar">
          <div className="nx-console-sidebar__brand">
            <Logo size="sm" />
            <div>
              <p className="nx-kicker nx-console-sidebar__kicker">CLOUD CONTROL</p>
              <p className="nx-console-sidebar__title">{t("nav.console")}</p>
            </div>
          </div>
          <nav className="nx-console-nav" aria-label={t("console.aria")}>
            {MENU.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={"end" in item ? item.end : false}
                className={({ isActive }) =>
                  `nx-console-nav__link${isActive ? " is-active" : ""}`
                }
              >
                <span>{item.code}</span>
                <strong>{t(item.labelKey)}</strong>
              </NavLink>
            ))}
          </nav>
          <div className="nx-console-sidebar__link-status">
            <span><i aria-hidden /> {t("console.connected")}</span>
            <strong>{t("console.secureSession")}</strong>
            <small>anycode.work / TLS</small>
          </div>
        </aside>
        <div className="nx-console-main">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
