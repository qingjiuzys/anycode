import { Icon } from "@/components/Icon";
import { useTheme } from "@/hooks/useTheme";
import { useT } from "@/i18n/context";

export function ThemeToggle() {
  const t = useT();
  const { theme, toggle } = useTheme();
  const isDark = theme === "dark";

  return (
    <button
      type="button"
      className="dw-btn-ghost p-2"
      title={isDark ? t("layout.themeLight") : t("layout.themeDark")}
      onClick={toggle}
      aria-pressed={isDark}
    >
      <Icon name={isDark ? "light_mode" : "dark_mode"} size={20} />
    </button>
  );
}
