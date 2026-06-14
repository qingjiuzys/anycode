import { AppearanceSettingsPanel } from "@/components/settings/AppearanceSettingsPanel";
import { AppleMediaSettingsPanel } from "@/components/settings/AppleMediaSettingsPanel";
import { AssetReadPolicyPanel } from "@/components/settings/AssetReadPolicyPanel";
import { PromptPreviewPanel } from "@/components/settings/PromptPreviewPanel";
import { ReportPreferencesPanel } from "@/components/settings/ReportPreferencesPanel";
import { UiDensityPanel } from "@/components/settings/UiDensityPanel";
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
      <AppearanceSettingsPanel />
      <AppleMediaSettingsPanel />
      <ReportPreferencesPanel />
      <PromptPreviewPanel />
      <AssetReadPolicyPanel />
      <UiDensityPanel />
    </>
  );
}
