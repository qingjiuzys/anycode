import { AssetReadPolicyPanel } from "@/components/settings/AssetReadPolicyPanel";
import { PromptPreviewPanel } from "@/components/settings/PromptPreviewPanel";
import { ReportPreferencesPanel } from "@/components/settings/ReportPreferencesPanel";
import { UiDensityPanel } from "@/components/settings/UiDensityPanel";
import { SkinPickerPanel } from "@/components/SkinPicker";
import { SectionCard } from "@/components/ui/SectionCard";
import { useDashboardPreferences } from "@/hooks/useDashboardPreferences";
import { useT } from "@/i18n/context";

export function SettingsPreferencesSection() {
  const t = useT();
  const { view, query } = useDashboardPreferences();

  return (
    <>
      <SectionCard title={t("settings.userPrefs.introTitle")}>
        <p className="text-sm text-secondary m-0">{t("settings.userPrefs.introHint")}</p>
        {view?.preferences_path ? (
          <p className="text-xs text-secondary m-0 mt-3">
            {t("settings.prefs.file")}:{" "}
            <code className="font-code">{view.preferences_path}</code>
          </p>
        ) : null}
        {query.isError && (
          <p className="text-sm text-error m-0 mt-3">{(query.error as Error).message}</p>
        )}
      </SectionCard>
      <SectionCard title={t("settings.skin.title")}>
        <p className="text-sm text-secondary m-0 mb-4">{t("settings.skin.hint")}</p>
        <SkinPickerPanel />
      </SectionCard>
      <ReportPreferencesPanel />
      <PromptPreviewPanel />
      <AssetReadPolicyPanel />
      <UiDensityPanel />
    </>
  );
}
