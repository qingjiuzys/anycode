import { useEffect, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { PageHeader } from "@/components/ui/PageHeader";
import { ConsoleShell } from "@/components/service/ConsoleShell";
import type { ServiceSection } from "@/components/service/ServiceNav";
import { ServiceCloudShell } from "@/components/service/ServiceCloudShell";
import { ServiceMockBanner } from "@/components/service/UpgradePromptCard";
import { ServiceOverviewSection } from "@/components/service/ServiceOverviewSection";
import { ServicePlanSection } from "@/components/service/ServicePlanSection";
import { ServiceUsageSection } from "@/components/service/ServiceUsageSection";
import { ServiceBillingSection } from "@/components/service/ServiceBillingSection";
import { ServiceApiSection } from "@/components/service/ServiceApiSection";
import { ServiceEnterpriseSection } from "@/components/service/ServiceEnterpriseSection";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";

const VALID_SECTIONS = new Set<ServiceSection>([
  "overview",
  "plan",
  "usage",
  "billing",
  "api",
  "enterprise",
]);

function parseSection(raw: unknown): ServiceSection {
  if (typeof raw === "string" && VALID_SECTIONS.has(raw as ServiceSection)) {
    return raw as ServiceSection;
  }
  return "overview";
}

export function ServicePage({ embedded }: EmbeddedPageProps = {}) {
  if (embedded) return <ServicePageInner syncUrl={false} />;
  return <ServicePageRouted />;
}

function ServicePageRouted() {
  const { section: sectionSearch } = useSearch({ from: "/_shell/account" });
  return <ServicePageInner initialSection={sectionSearch} syncUrl />;
}

function ServicePageInner({
  initialSection,
  syncUrl = true,
}: {
  initialSection?: ServiceSection;
  syncUrl?: boolean;
}) {
  const t = useT();
  const navigate = useNavigate();
  const [section, setSection] = useState<ServiceSection>(() => parseSection(initialSection));

  useEffect(() => {
    if (initialSection !== undefined) {
      setSection(parseSection(initialSection));
    }
  }, [initialSection]);

  const onSectionChange = (next: ServiceSection) => {
    setSection(next);
    if (syncUrl) {
      navigate({ to: "/account", search: { section: next }, replace: true });
    }
  };

  return (
    <>
      <PageHeader
        title={t("service.console.pageTitle")}
        subtitle={t("service.console.pageSubtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("service.console.pageTitle") },
        ]}
      />

      <ServiceCloudShell>
        <ConsoleShell active={section} onSectionChange={onSectionChange}>
          <ServiceMockBanner />

          <div className="space-y-6">
            {section === "overview" && <ServiceOverviewSection />}
            {section === "plan" && <ServicePlanSection />}
            {section === "usage" && <ServiceUsageSection />}
            {section === "billing" && <ServiceBillingSection />}
            {section === "api" && <ServiceApiSection />}
            {section === "enterprise" && <ServiceEnterpriseSection />}
          </div>
        </ConsoleShell>
      </ServiceCloudShell>
    </>
  );
}
