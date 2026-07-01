import { useEffect, useState } from "react";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { SettingsNav, type SettingsSection } from "@/components/settings/SettingsNav";
import { PageHeader } from "@/components/ui/PageHeader";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";
import { SettingsAgentsSection } from "@/pages/settings/SettingsAgentsSection";
import { SettingsAuthSection } from "@/pages/settings/SettingsAuthSection";
import { SettingsChannelsSection } from "@/pages/settings/SettingsChannelsSection";
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
  "security",
  "notify",
  "channels",
  "ops",
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
    <>
      <PageHeader
        title={t("settings.title")}
        subtitle={t("settings.subtitle")}
        breadcrumbs={[{ label: t("settings.title") }]}
        actions={
          <>
            <button type="button" className="dw-btn-secondary" onClick={() => window.history.back()}>
              <Icon name="chevron_left" size={16} />
              {t("common.back")}
            </button>
            <Link to="/projects" className="dw-btn-secondary no-underline">
              <Icon name="folder" size={16} />
              {t("nav.projects")}
            </Link>
          </>
        }
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
          {section === "security" && <SettingsSecuritySection />}
          {section === "notify" && <SettingsNotifySection />}
          {section === "channels" && <SettingsChannelsSection />}
          {section === "ops" && <SettingsOpsSection />}
        </div>
      </div>
    </>
  );
}
