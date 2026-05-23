import { ConnectorPanel } from "@/components/ConnectorPanel";
import { NotificationPoliciesPanel } from "@/components/settings/NotificationPoliciesPanel";

export function SettingsNotifySection() {
  return (
    <>
      <NotificationPoliciesPanel />
      <ConnectorPanel />
    </>
  );
}
