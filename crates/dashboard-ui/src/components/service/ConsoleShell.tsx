import type { ReactNode } from "react";
import { ConsoleQuotaCard } from "@/components/service/ConsoleQuotaCard";
import { ServiceNav, type ServiceSection } from "@/components/service/ServiceNav";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ConsoleShell({
  active,
  onSectionChange,
  children,
}: {
  active: ServiceSection;
  onSectionChange: (s: ServiceSection) => void;
  children: ReactNode;
}) {
  const t = useT();
  const { user, logout, openPortalLogin } = useAccountCloud();

  return (
    <div className="console-shell">
      <header className="console-topbar glass-panel">
        <div className="console-topbar-brand">
          <span className="console-logo" aria-hidden />
          <span className="font-semibold text-sm">818Cloud</span>
        </div>
        <div className="console-topbar-actions">
          <span className="text-xs text-secondary hidden sm:inline">
            {t("service.cloud.signedInAs")}{" "}
            <span className="text-on-surface font-medium">{user?.email}</span>
          </span>
          <button
            type="button"
            className="dw-btn-ghost text-xs"
            onClick={() => openPortalLogin("/plans")}
          >
            {t("service.cloud.manageInPortal")}
          </button>
          <button type="button" className="dw-btn-ghost text-xs" onClick={() => void logout()}>
            {t("service.cloud.signOutCloud")}
          </button>
        </div>
      </header>

      <div className="console-body">
        <aside className="console-sidebar glass-panel">
          <ServiceNav active={active} onChange={onSectionChange} variant="console" />
          <ConsoleQuotaCard onNavigate={onSectionChange} />
        </aside>
        <main className="console-main">{children}</main>
      </div>
    </div>
  );
}
