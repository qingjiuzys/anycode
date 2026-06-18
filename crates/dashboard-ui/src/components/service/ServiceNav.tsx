import { useT } from "@/i18n/context";

export type ServiceSection = "overview" | "plan" | "usage" | "billing" | "api" | "enterprise";

const WORKSPACE_SECTIONS: ServiceSection[] = ["overview", "usage"];
const ACCOUNT_SECTIONS: ServiceSection[] = ["plan", "billing", "api", "enterprise"];

export function ServiceNav({
  active,
  onChange,
  variant = "settings",
}: {
  active: ServiceSection;
  onChange: (s: ServiceSection) => void;
  variant?: "settings" | "console";
}) {
  const t = useT();
  const navClass =
    variant === "console" ? "console-nav" : "dw-settings-nav";
  const linkClass = (id: ServiceSection) =>
    variant === "console"
      ? `console-nav-link${active === id ? " active" : ""}`
      : `dw-settings-nav-link${active === id ? " active" : ""}`;

  if (variant === "console") {
    return (
      <nav className={navClass} aria-label={t("service.navLabel")}>
        <p className="console-nav-group">{t("service.console.nav.workspace")}</p>
        {WORKSPACE_SECTIONS.map((id) => (
          <button
            key={id}
            type="button"
            className={linkClass(id)}
            onClick={() => onChange(id)}
          >
            {t(`service.tabs.${id}`)}
          </button>
        ))}
        <p className="console-nav-group">{t("service.console.nav.account")}</p>
        {ACCOUNT_SECTIONS.map((id) => (
          <button
            key={id}
            type="button"
            className={linkClass(id)}
            onClick={() => onChange(id)}
          >
            {t(`service.tabs.${id}`)}
          </button>
        ))}
      </nav>
    );
  }

  const sections: ServiceSection[] = [...WORKSPACE_SECTIONS, ...ACCOUNT_SECTIONS];
  return (
    <nav className={navClass} aria-label={t("service.navLabel")}>
      {sections.map((id) => (
        <button
          key={id}
          type="button"
          className={linkClass(id)}
          onClick={() => onChange(id)}
        >
          {t(`service.tabs.${id}`)}
        </button>
      ))}
    </nav>
  );
}
