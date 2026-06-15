import { SectionCard } from "@/components/ui/SectionCard";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceCloudLogin() {
  const t = useT();
  const { portalUrl, openPortalLogin } = useAccountCloud();

  return (
    <SectionCard title={t("service.cloud.signInTitle")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("service.cloud.portalHint")}</p>
      {portalUrl && (
        <p className="text-xs font-code text-secondary m-0 mb-4 break-all">{portalUrl}</p>
      )}
      <div className="flex flex-wrap gap-2">
        <button type="button" className="dw-btn-primary text-sm" onClick={() => openPortalLogin()}>
          {t("service.cloud.openPortalLogin")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          onClick={() => openPortalLogin("/devices")}
        >
          {t("service.cloud.openPortalDevices")}
        </button>
      </div>
    </SectionCard>
  );
}
