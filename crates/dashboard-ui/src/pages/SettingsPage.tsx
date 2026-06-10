import { useEffect, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { SettingsNav, type SettingsSection } from "@/components/settings/SettingsNav";
import { PageHeader } from "@/components/ui/PageHeader";
import { useT } from "@/i18n/context";
import { SettingsAgentsSection } from "@/pages/settings/SettingsAgentsSection";
import { SettingsAuthSection } from "@/pages/settings/SettingsAuthSection";
import { SettingsAssetsSection } from "@/pages/settings/SettingsAssetsSection";
import { SettingsDataSection } from "@/pages/settings/SettingsDataSection";
import { SettingsModelSection } from "@/pages/settings/SettingsModelSection";
import { SettingsNotifySection } from "@/pages/settings/SettingsNotifySection";
import { SettingsOpsSection } from "@/pages/settings/SettingsOpsSection";
import { SettingsPreferencesSection } from "@/pages/settings/SettingsPreferencesSection";
import { SettingsOverviewBanner } from "@/pages/settings/SettingsOverviewBanner";
import { SettingsSecuritySection } from "@/pages/settings/SettingsSecuritySection";
import { SettingsServiceSection } from "@/pages/settings/SettingsServiceSection";
import { SettingsSkillsSection } from "@/pages/settings/SettingsSkillsSection";

const VALID_SECTIONS = new Set<SettingsSection>([
  "auth",
  "prefs",
  "data",
  "service",
  "model",
  "agents",
  "skills",
  "assets",
  "security",
  "notify",
  "ops",
]);

function parseSettingsSection(raw: unknown): SettingsSection {
  if (typeof raw === "string" && VALID_SECTIONS.has(raw as SettingsSection)) {
    return raw as SettingsSection;
  }
  return "auth";
}

export function SettingsPage() {
  const t = useT();
  const navigate = useNavigate();
  const { section: sectionSearch } = useSearch({ from: "/_shell/settings" });
  const [section, setSection] = useState<SettingsSection>(() => parseSettingsSection(sectionSearch));

  useEffect(() => {
    setSection(parseSettingsSection(sectionSearch));
  }, [sectionSearch]);

  const onSectionChange = (next: SettingsSection) => {
    setSection(next);
    navigate({ to: "/settings", search: { section: next }, replace: true });
  };

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
        <SettingsNav active={section} onChange={onSectionChange} />

        <div className="dw-settings-content space-y-6">
          {section === "auth" && <SettingsAuthSection />}
          {section === "prefs" && <SettingsPreferencesSection />}
          {section === "data" && <SettingsDataSection />}
          {section === "service" && <SettingsServiceSection />}
          {section === "model" && <SettingsModelSection />}
          {section === "agents" && <SettingsAgentsSection />}
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
