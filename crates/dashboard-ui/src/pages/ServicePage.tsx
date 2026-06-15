import { useEffect, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { PageHeader } from "@/components/ui/PageHeader";
import { ServiceNav, type ServiceSection } from "@/components/service/ServiceNav";
import { ServiceCloudShell } from "@/components/service/ServiceCloudShell";
import { ServiceMockBanner } from "@/components/service/UpgradePromptCard";
import { ServicePlanSection } from "@/components/service/ServicePlanSection";
import { ServiceUsageSection } from "@/components/service/ServiceUsageSection";
import { ServiceBillingSection } from "@/components/service/ServiceBillingSection";
import { ServiceApiSection } from "@/components/service/ServiceApiSection";
import { ServiceEnterpriseSection } from "@/components/service/ServiceEnterpriseSection";
import { useT } from "@/i18n/context";

const VALID_SECTIONS = new Set<ServiceSection>(["plan", "usage", "billing", "api", "enterprise"]);

function parseSection(raw: unknown): ServiceSection {
  if (typeof raw === "string" && VALID_SECTIONS.has(raw as ServiceSection)) {
    return raw as ServiceSection;
  }
  return "plan";
}

export function ServicePage() {
  const t = useT();
  const navigate = useNavigate();
  const { section: sectionSearch } = useSearch({ from: "/_shell/account" });
  const [section, setSection] = useState<ServiceSection>(() => parseSection(sectionSearch));

  useEffect(() => {
    setSection(parseSection(sectionSearch));
  }, [sectionSearch]);

  const onSectionChange = (next: ServiceSection) => {
    setSection(next);
    navigate({ to: "/account", search: { section: next }, replace: true });
  };

  return (
    <>
      <PageHeader
        title={t("service.title")}
        subtitle={t("service.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("service.title") },
        ]}
      />

      <ServiceCloudShell>
        <ServiceMockBanner />

        <div className="dw-settings">
          <ServiceNav active={section} onChange={onSectionChange} />

          <div className="dw-settings-content space-y-6">
            {section === "plan" && <ServicePlanSection />}
            {section === "usage" && <ServiceUsageSection />}
            {section === "billing" && <ServiceBillingSection />}
            {section === "api" && <ServiceApiSection />}
            {section === "enterprise" && <ServiceEnterpriseSection />}
          </div>
        </div>
      </ServiceCloudShell>
    </>
  );
}
