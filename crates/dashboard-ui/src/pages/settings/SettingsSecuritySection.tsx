import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { PolicySummaryPanel } from "@/components/PolicySummary";
import { ToolGovernancePanel } from "@/components/settings/ToolGovernancePanel";

export function SettingsSecuritySection() {
  const policies = useQuery({ queryKey: ["policies"], queryFn: api.policies });
  const toolGovernance = useQuery({
    queryKey: ["tool-governance"],
    queryFn: api.toolGovernance,
  });

  return (
    <>
      <PolicySummaryPanel policy={policies.data?.policy} />
      <ToolGovernancePanel governance={toolGovernance.data} />
    </>
  );
}
