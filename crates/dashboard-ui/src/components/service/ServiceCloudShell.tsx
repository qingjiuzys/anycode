import type { ReactNode } from "react";
import { ServiceCloudLogin } from "@/components/service/ServiceCloudLogin";
import { ServiceNotConfigured } from "@/components/service/ServiceNotConfigured";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceCloudShell({ children }: { children: ReactNode }) {
  const t = useT();
  const { configured, authenticated, loading, user, logout } = useAccountCloud();

  if (!configured) {
    return <ServiceNotConfigured />;
  }

  if (loading) {
    return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  }

  if (!authenticated) {
    return <ServiceCloudLogin />;
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
        <span className="text-secondary">
          {t("service.cloud.signedInAs")}{" "}
          <span className="text-on-surface font-medium">{user?.email}</span>
        </span>
        <button type="button" className="dw-btn-ghost text-sm" onClick={() => void logout()}>
          {t("service.cloud.signOutCloud")}
        </button>
      </div>
      {children}
    </div>
  );
}
