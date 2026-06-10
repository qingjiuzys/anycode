import { ConnectorPanel } from "@/components/ConnectorPanel";
import { BrowserConnectorPanel } from "@/components/settings/BrowserConnectorPanel";
import { McpServersPanel } from "@/components/settings/McpServersPanel";
import { NotificationPoliciesPanel } from "@/components/settings/NotificationPoliciesPanel";

export function SettingsNotifySection() {
  return (
    <>
      <BrowserConnectorPanel />
      <McpServersPanel />
      <NotificationPoliciesPanel />
      <ConnectorPanel />
    </>
  );
}
