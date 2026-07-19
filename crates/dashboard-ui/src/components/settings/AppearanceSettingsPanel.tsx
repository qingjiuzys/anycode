import { SkinPickerPanel } from "@/components/SkinPicker";
import { ThemeModeSwitch } from "@/components/ThemeModeSwitch";
import { SectionCard } from "@/components/ui/SectionCard";
import { useI18n, useT } from "@/i18n/context";

const LOCALES = [
  { id: "zh" as const, labelKey: "common.zh" },
  { id: "en" as const, labelKey: "common.en" },
];

export function AppearanceSettingsPanel() {
  const t = useT();
  const { locale, setLocale } = useI18n();

  return (
    <SectionCard title={t("settings.appearance.title")}>
      <p className="text-sm text-secondary m-0 mb-5">{t("settings.appearance.hint")}</p>

      <div className="appearance-settings-block">
        <div className="appearance-settings-block__head">
          <h4 className="appearance-settings-block__title">{t("settings.appearance.languageLabel")}</h4>
          <p className="appearance-settings-block__hint">{t("settings.appearance.languageHint")}</p>
        </div>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("common.language")}>
          {LOCALES.map((item) => (
            <button
              key={item.id}
              type="button"
              className={
                locale === item.id ? "dw-btn-primary text-sm" : "dw-btn-secondary text-sm"
              }
              aria-pressed={locale === item.id}
              onClick={() => setLocale(item.id)}
            >
              {t(item.labelKey)}
            </button>
          ))}
        </div>
      </div>

      <div className="appearance-settings-block mt-6">
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
