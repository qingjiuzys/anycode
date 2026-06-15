import { useT } from "@/i18n/context";

export type ServiceSection = "plan" | "usage" | "billing" | "api" | "enterprise";

const SECTIONS: ServiceSection[] = ["plan", "usage", "billing", "api", "enterprise"];

export function ServiceNav({
  active,
  onChange,
}: {
  active: ServiceSection;
  onChange: (s: ServiceSection) => void;
}) {
  const t = useT();
  return (
    <nav className="dw-settings-nav" aria-label={t("service.navLabel")}>
      {SECTIONS.map((id) => (
        <button
          key={id}
          type="button"
          className={`dw-settings-nav-link${active === id ? " active" : ""}`}
          onClick={() => onChange(id)}
        >
          {t(`service.tabs.${id}`)}
        </button>
      ))}
    </nav>
  );
}
