import { Link } from "@tanstack/react-router";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SettingsAssetsSection() {
  const t = useT();

  return (
    <SectionCard title={t("settings.assetsRedirect.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.assetsRedirect.hint")}</p>
      <Link to="/settings" search={{ section: "prefs" }} className="dw-btn-secondary no-underline">
        {t("settings.assetsRedirect.openPrefs")}
      </Link>
    </SectionCard>
  );
}
