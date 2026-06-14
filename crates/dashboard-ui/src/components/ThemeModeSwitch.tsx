import { Icon } from "@/components/Icon";
import { useTheme, type Theme } from "@/hooks/useTheme";
import { useT } from "@/i18n/context";

export function ThemeModeSwitch({ className }: { className?: string }) {
  const t = useT();
  const { theme, setTheme } = useTheme();

  const options: { id: Theme; label: string; icon: string }[] = [
    { id: "light", label: t("layout.themeLight"), icon: "light_mode" },
    { id: "dark", label: t("layout.themeDark"), icon: "dark_mode" },
  ];

  return (
    <div
      className={`appearance-theme-switch ${className ?? ""}`}
      role="group"
      aria-label={t("settings.appearance.themeLabel")}
    >
      {options.map((opt) => (
        <button
          key={opt.id}
          type="button"
          className={`appearance-theme-switch__btn ${theme === opt.id ? "active" : ""}`}
          aria-pressed={theme === opt.id}
          onClick={() => setTheme(opt.id)}
        >
          <Icon name={opt.icon} size={16} />
          <span>{opt.label}</span>
        </button>
      ))}
    </div>
  );
}
