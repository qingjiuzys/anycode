import type { ReactNode } from "react";
import { ServiceCloudLogin } from "@/components/service/ServiceCloudLogin";
import { ServiceNotConfigured } from "@/components/service/ServiceNotConfigured";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceCloudShell({ children }: { children: ReactNode }) {
  const t = useT();
  const { configured, authenticated, loading } = useAccountCloud();

  if (!configured) {
    return <ServiceNotConfigured />;
  }

  if (loading) {
    return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  }

  if (!authenticated) {
    return <ServiceCloudLogin />;
  }

  return <>{children}</>;
}
