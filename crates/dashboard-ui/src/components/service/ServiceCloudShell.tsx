import type { ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { EmptyState } from "@/components/EmptyState";
import { ServiceNotConfigured } from "@/components/service/ServiceNotConfigured";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceCloudShell({ children }: { children: ReactNode }) {
  const t = useT();
  const { configured, cloudLinked, loading } = useAccountCloud();

  if (!configured) {
    return <ServiceNotConfigured />;
  }

  if (loading) {
    return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  }

  if (!cloudLinked) {
    return (
      <EmptyState
        icon="cloud_off"
        title={t("service.cloud.connectTitle")}
        description={t("service.cloud.connectBody")}
        actions={
          <Link to="/cloud-login" className="dw-btn-primary no-underline">
            {t("service.cloud.linkAccount")}
          </Link>
        }
      />
    );
  }

  return <>{children}</>;
}
