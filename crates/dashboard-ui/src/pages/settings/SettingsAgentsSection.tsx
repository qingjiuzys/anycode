import { AgentSettingsPanel } from "@/components/settings/AgentSettingsPanel";
import { AgentLoopLimitsPanel } from "@/components/settings/AgentLoopLimitsPanel";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SettingsAgentsSection() {
  const t = useT();
  return (
    <div className="space-y-6">
      <SectionCard title={t("settings.agents.title")}>
        <p className="text-sm text-secondary m-0">{t("settings.agents.subtitle")}</p>
      </SectionCard>
      <AgentLoopLimitsPanel />
      <AgentSettingsPanel />
    </div>
  );
}
