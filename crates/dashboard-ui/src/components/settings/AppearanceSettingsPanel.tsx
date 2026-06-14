import { SkinPickerPanel } from "@/components/SkinPicker";
import { ThemeModeSwitch } from "@/components/ThemeModeSwitch";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function AppearanceSettingsPanel() {
  const t = useT();

  return (
    <SectionCard title={t("settings.appearance.title")}>
      <p className="text-sm text-secondary m-0 mb-5">{t("settings.appearance.hint")}</p>

      <div className="appearance-settings-block">
        <div className="appearance-settings-block__head">
          <h4 className="appearance-settings-block__title">{t("settings.appearance.themeLabel")}</h4>
          <p className="appearance-settings-block__hint">{t("settings.appearance.themeHint")}</p>
        </div>
        <ThemeModeSwitch className="appearance-settings-block__control" />
      </div>

      <div className="appearance-settings-block mt-6">
        <div className="appearance-settings-block__head">
          <h4 className="appearance-settings-block__title">{t("settings.appearance.skinLabel")}</h4>
          <p className="appearance-settings-block__hint">{t("settings.appearance.skinHint")}</p>
        </div>
        <SkinPickerPanel />
      </div>
    </SectionCard>
  );
}
