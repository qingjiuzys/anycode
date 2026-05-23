import { AuthOrgPanel } from "@/components/settings/AuthOrgPanel";
import { TokenPanel } from "@/components/TokenPanel";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";

export function SettingsAuthSection() {
  const runtime = useRuntimeSettings();
  const rt = runtime.data?.runtime;

  return (
    <>
      <AuthOrgPanel runtime={rt} />
      <TokenPanel />
    </>
  );
}
