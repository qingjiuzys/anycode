import { EmptyState } from "@/components/EmptyState";
import { useT } from "@/i18n/context";

export function ServiceNotConfigured() {
  const t = useT();
  return (
    <EmptyState
      icon="corporate_fare"
      title={t("service.cloud.notConfiguredTitle")}
      description={t("service.cloud.notConfiguredBody")}
    />
  );
}
