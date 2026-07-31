import { useT } from "@/i18n/context";

export type SettingsSection =
  | "prefs"
  | "data"
  | "service"
  | "model"
  | "agents"
  | "skills"
  | "security"
  | "notify"
  | "gates"
  | "plugins"
  | "ops"
  | "about";

export const SETTINGS_SECTIONS: SettingsSection[] = [
  "prefs",
  "data",
  "service",
  "model",
  "agents",
  "skills",
  "security",
  "notify",
  "gates",
  "plugins",
  "ops",
  "about",
];

export function SettingsNav({
  active,
  onChange,
}: {
  active: SettingsSection;
  onChange: (s: SettingsSection) => void;
}) {
  const t = useT();
  return (
    <nav className="dw-settings-nav">
      {SETTINGS_SECTIONS.map((id) => (
        <button
          key={id}
          type="button"
          className={`dw-settings-nav-link${active === id ? " active" : ""}`}
          onClick={() => onChange(id)}
        >
          {t(`settings.tabs.${id}`)}
        </button>
      ))}
    </nav>
  );
}
