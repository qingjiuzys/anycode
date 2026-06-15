import { CloudApiKeyPanel } from "@/components/service/CloudApiKeyPanel";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function ServiceApiSection() {
  const t = useT();

  return (
    <div className="space-y-6">
      <SectionCard title={t("service.api.title")}>
        <p className="text-sm text-secondary m-0 mb-2">{t("service.api.subtitle")}</p>
        <p className="text-xs text-secondary m-0">{t("service.api.notLlmKey")}</p>
      </SectionCard>
      <CloudApiKeyPanel />
    </div>
  );
}
