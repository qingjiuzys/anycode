import { ModelConfigForm } from "@/components/settings/ModelConfigForm";
import { RuntimeConfigPanel } from "@/components/settings/RuntimeConfigPanel";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";

export function SettingsModelSection() {
  const runtime = useRuntimeSettings();
  const rt = runtime.data?.runtime;

  return (
    <>
      <ModelConfigForm runtime={rt} />
      <RuntimeConfigPanel runtime={rt} />
    </>
  );
}
