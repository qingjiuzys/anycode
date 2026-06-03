import { ModelManagerPanel } from "@/components/settings/ModelManagerPanel";
import { RuntimeConfigPanel } from "@/components/settings/RuntimeConfigPanel";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";

export function SettingsModelSection() {
  const runtime = useRuntimeSettings();
  const rt = runtime.data?.runtime;

  return (
    <>
      <ModelManagerPanel />
      <RuntimeConfigPanel runtime={rt} />
    </>
  );
}
