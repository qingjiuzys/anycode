import { SkillsGovernancePanel } from "@/components/settings/SkillsGovernancePanel";
import { SkillsImportPanel } from "@/components/settings/SkillsImportPanel";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";

export function SettingsSkillsSection() {
  const runtime = useRuntimeSettings();
  const rt = runtime.data?.runtime;

  return (
    <>
      <SkillsImportPanel />
      <SkillsGovernancePanel runtime={rt} />
    </>
  );
}
