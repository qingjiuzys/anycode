import { useState } from "react";
import { SettingsNav, type SettingsSection } from "@/components/settings/SettingsNav";
import { PageHeader } from "@/components/ui/PageHeader";
import { useT } from "@/i18n/context";
import { SettingsAuthSection } from "@/pages/settings/SettingsAuthSection";
import { SettingsAssetsSection } from "@/pages/settings/SettingsAssetsSection";
import { SettingsDataSection } from "@/pages/settings/SettingsDataSection";
import { SettingsModelSection } from "@/pages/settings/SettingsModelSection";
import { SettingsNotifySection } from "@/pages/settings/SettingsNotifySection";
import { SettingsOpsSection } from "@/pages/settings/SettingsOpsSection";
import { SettingsOverviewBanner } from "@/pages/settings/SettingsOverviewBanner";
import { SettingsSecuritySection } from "@/pages/settings/SettingsSecuritySection";
import { SettingsServiceSection } from "@/pages/settings/SettingsServiceSection";
import { SettingsSkillsSection } from "@/pages/settings/SettingsSkillsSection";

export function SettingsPage() {
  const t = useT();
  const [section, setSection] = useState<SettingsSection>("auth");

  return (
    <>
      <PageHeader
        title={t("settings.title")}
        subtitle={t("settings.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("settings.title") },
        ]}
      />

      <SettingsOverviewBanner />

      <div className="dw-settings">
        <SettingsNav active={section} onChange={setSection} />

        <div className="dw-settings-content space-y-6">
          {section === "auth" && <SettingsAuthSection />}
          {section === "data" && <SettingsDataSection />}
          {section === "service" && <SettingsServiceSection />}
          {section === "model" && <SettingsModelSection />}
          {section === "skills" && <SettingsSkillsSection />}
          {section === "assets" && <SettingsAssetsSection />}
          {section === "security" && <SettingsSecuritySection />}
          {section === "notify" && <SettingsNotifySection />}
          {section === "ops" && <SettingsOpsSection />}
        </div>
      </div>
    </>
  );
}
