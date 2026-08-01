import type { ReactNode } from "react";
import { Icon } from "@/components/Icon";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

/** When linked but entitlements are missing, show loading / error instead of a blank main pane. */
export function ConsoleEntitlementsGate({ children }: { children: ReactNode }) {
  const t = useT();
  const {
    entitlements,
    entitlementsLoading,
    entitlementsError,
    refresh,
    linkCloudAccount,
    linking,
  } = useAccountCloud();

  if (entitlements) {
    return <>{children}</>;
  }

  if (entitlementsLoading) {
    return (
      <div className="console-entitlements-state" role="status">
        <p className="text-sm text-secondary m-0">{t("service.console.entitlementsLoading")}</p>
      </div>
    );
  }

  if (entitlementsError) {
    const lower = entitlementsError.toLowerCase();
    const isSessionExpired =
      lower.includes("cloud_session_expired") ||
      lower.includes("session expired") ||
      lower.includes("invalid refresh") ||
      lower.includes("re-link your");
    const isNetwork =
      !isSessionExpired &&
      (lower.includes("load failed") ||
        lower.includes("failed to fetch") ||
        lower.includes("networkerror") ||
        lower.includes("upstream request failed"));
    const friendly =
      entitlementsError === "missing_token"
        ? t("service.console.entitlementsMissingToken")
        : entitlementsError === "bundle_empty"
          ? t("service.console.entitlementsEmpty")
          : isSessionExpired
            ? t("service.console.entitlementsSessionExpired")
            : isNetwork
              ? t("service.console.entitlementsNetwork")
              : t("service.console.entitlementsError");
    const showDetail =
      entitlementsError !== "missing_token" && entitlementsError !== "bundle_empty";
    return (
      <div className="console-entitlements-state console-entitlements-state--error" role="alert">
        <p className="text-sm text-on-surface font-medium m-0 mb-1">{friendly}</p>
        {showDetail && (
          <details className="mb-3">
            <summary className="text-[12px] text-secondary cursor-pointer">
              {t("settings.connectorErrDetails")}
            </summary>
            <p className="text-[12px] text-secondary m-0 mt-1 break-all font-code">
              {entitlementsError}
            </p>
          </details>
        )}
        <div className="flex flex-wrap gap-2">
          {isSessionExpired || entitlementsError === "missing_token" ? (
            <button
              type="button"
              className="dw-btn-primary text-sm"
              disabled={linking}
              onClick={() => void linkCloudAccount().catch(() => undefined)}
            >
              <Icon name="link" size={16} className="inline mr-1" />
              {linking ? t("service.cloud.linking") : t("service.console.entitlementsRelink")}
            </button>
          ) : (
            <button type="button" className="dw-btn-secondary text-sm" onClick={() => refresh()}>
              <Icon name="refresh" size={16} className="inline mr-1" />
              {t("common.retry")}
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="console-entitlements-state" role="status">
      <p className="text-sm text-secondary m-0">{t("service.console.entitlementsLoading")}</p>
    </div>
  );
}
