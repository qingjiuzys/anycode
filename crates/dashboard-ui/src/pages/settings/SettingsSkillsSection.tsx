import { SkillsGovernancePanel } from "@/components/settings/SkillsGovernancePanel";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";

export function SettingsSkillsSection() {
  const runtime = useRuntimeSettings();
  const rt = runtime.data?.runtime;

  return <SkillsGovernancePanel runtime={rt} />;
}
