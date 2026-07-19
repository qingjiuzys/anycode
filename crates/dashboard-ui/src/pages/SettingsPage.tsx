import { useEffect, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  SettingsNav,
  SETTINGS_SECTIONS,
  type SettingsSection,
} from "@/components/settings/SettingsNav";
import { PageHeader } from "@/components/ui/PageHeader";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";
import { SettingsAboutSection } from "@/pages/settings/SettingsAboutSection";
import { SettingsAgentsSection } from "@/pages/settings/SettingsAgentsSection";
import { SettingsChannelsSection } from "@/pages/settings/SettingsChannelsSection";
import { SettingsDataSection } from "@/pages/settings/SettingsDataSection";
import { SettingsGatesSection } from "@/pages/settings/SettingsGatesSection";
import { SettingsModelSection } from "@/pages/settings/SettingsModelSection";
import { SettingsNotifySection } from "@/pages/settings/SettingsNotifySection";
import { SettingsOpsSection } from "@/pages/settings/SettingsOpsSection";
import { SettingsPreferencesSection } from "@/pages/settings/SettingsPreferencesSection";
import { SettingsOverviewBanner } from "@/pages/settings/SettingsOverviewBanner";
import { SettingsSecuritySection } from "@/pages/settings/SettingsSecuritySection";
import { SettingsServiceSection } from "@/pages/settings/SettingsServiceSection";
import { SettingsSkillsSection } from "@/pages/settings/SettingsSkillsSection";
import { SettingsPluginsSection } from "@/pages/settings/SettingsPluginsSection";

const VALID_SECTIONS = new Set<SettingsSection>([
  "prefs",
  "data",
  "service",
  "model",
  "agents",
  "skills",
  "security",
  "notify",
  "channels",
  "gates",
  "plugins",
  "ops",
  "about",
]);

function parseSettingsSection(raw: unknown): SettingsSection {
  if (typeof raw === "string" && VALID_SECTIONS.has(raw as SettingsSection)) {
    return raw as SettingsSection;
  }
  return "prefs";
}

export function SettingsPage({ embedded, initialSearch }: EmbeddedPageProps = {}) {
  if (embedded) {
    return (
      <SettingsPageInner
        initialSection={parseSettingsSection(initialSearch?.section)}
        syncUrl={false}
      />
    );
  }
  return <SettingsPageRouted />;
}

function SettingsPageRouted() {
  const { section: sectionSearch } = useSearch({ from: "/_shell/settings" });
  return <SettingsPageInner initialSection={sectionSearch} syncUrl />;
}

function SettingsPageInner({
  initialSection,
  syncUrl = true,
}: {
  initialSection?: SettingsSection;
  syncUrl?: boolean;
}) {
  const t = useT();
  const navigate = useNavigate();
  const [section, setSection] = useState<SettingsSection>(() =>
    parseSettingsSection(initialSection),
  );

  useEffect(() => {
    if (initialSection !== undefined) {
      setSection(parseSettingsSection(initialSection));
    }
  }, [initialSection]);

  const onSectionChange = (next: SettingsSection) => {
    setSection(next);
    if (syncUrl) {
      navigate({ to: "/settings", search: { section: next }, replace: true });
    }
  };

  return (
    <div className="dw-settings-page">
      <div className="dw-settings-page-header">
        <PageHeader
          title={t("settings.title")}
          subtitle={t("settings.subtitle")}
          breadcrumbs={[{ label: t("settings.title") }]}
        />
        <SettingsOverviewBanner />
      </div>

      <div className="dw-settings">
        <SettingsNav active={section} onChange={onSectionChange} />

        <div className="dw-settings-content">
          {/* Mobile only — desktop uses left SettingsNav. */}
          <div className="dw-settings-content-toolbar lg:hidden">
            <label className="dw-settings-section-picker">
              <span className="dw-settings-section-picker-label">{t("settings.sectionSelect")}</span>
              <select
                className="dw-input dw-settings-section-select"
                value={section}
                onChange={(e) => onSectionChange(e.target.value as SettingsSection)}
                aria-label={t("settings.sectionSelect")}
              >
                {SETTINGS_SECTIONS.map((id) => (
                  <option key={id} value={id}>
                    {t(`settings.tabs.${id}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="dw-settings-content-body space-y-6">
            {section === "prefs" && <SettingsPreferencesSection />}
            {section === "data" && <SettingsDataSection />}
            {section === "service" && <SettingsServiceSection />}
            {section === "model" && <SettingsModelSection />}
            {section === "agents" && <SettingsAgentsSection />}
            {section === "skills" && <SettingsSkillsSection />}
            {section === "security" && <SettingsSecuritySection />}
            {section === "notify" && <SettingsNotifySection />}
            {section === "channels" && <SettingsChannelsSection />}
            {section === "gates" && <SettingsGatesSection />}
            {section === "plugins" && <SettingsPluginsSection />}
            {section === "ops" && <SettingsOpsSection />}
            {section === "about" && <SettingsAboutSection />}
          </div>
        </div>
      </div>
    </div>
  );
}
