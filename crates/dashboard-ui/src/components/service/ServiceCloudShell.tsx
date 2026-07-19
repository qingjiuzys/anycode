import type { ReactNode } from "react";
import { Navigate } from "@tanstack/react-router";
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
    // Prefer the dedicated frontmost gate over an embedded console card.
    return <Navigate to="/cloud-login" replace />;
  }

  return <>{children}</>;
}
