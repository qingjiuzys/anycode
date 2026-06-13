import { useT } from "@/i18n/context";

export type SettingsSection =
  | "auth"
  | "prefs"
  | "data"
  | "service"
  | "model"
  | "agents"
  | "skills"
  | "security"
  | "notify"
  | "channels"
  | "ops";

const SECTIONS: SettingsSection[] = [
  "auth",
  "prefs",
  "data",
  "service",
  "model",
  "agents",
  "skills",
  "security",
  "notify",
  "channels",
  "ops",
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
      {SECTIONS.map((id) => (
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
