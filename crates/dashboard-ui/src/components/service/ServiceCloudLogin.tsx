import { SectionCard } from "@/components/ui/SectionCard";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceCloudLogin() {
  const t = useT();
  const { portalUrl, linkCloudAccount, linking, linkError, openPortalLogin } = useAccountCloud();

  return (
    <SectionCard title={t("service.cloud.signInTitle")}>
      <p className="text-sm text-secondary m-0 mb-2">{t("service.cloud.signInBody")}</p>
      <p className="text-sm text-secondary m-0 mb-4">{t("service.cloud.portalHint")}</p>
      {portalUrl && (
        <p className="text-xs font-code text-secondary m-0 mb-4 break-all">{portalUrl}</p>
      )}
      {linkError && (
        <p className="text-sm text-error m-0 mb-3" role="alert">
          {t("service.cloud.linkFailed")}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="dw-btn-primary text-sm"
          disabled={linking}
          onClick={() => void linkCloudAccount().catch(() => undefined)}
        >
          {linking ? t("service.cloud.linking") : t("service.cloud.linkAccount")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          disabled={linking}
          onClick={() => openPortalLogin()}
        >
          {t("service.cloud.openPortalLogin")}
        </button>
      </div>
      <p className="text-xs text-secondary m-0 mt-4">{t("service.cloud.linkSteps")}</p>
    </SectionCard>
  );
}
