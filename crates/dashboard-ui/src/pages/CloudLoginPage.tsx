import { useEffect, useMemo } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { BrandMark } from "@/components/BrandMark";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useI18n } from "@/i18n/context";

/**
 * Cloud account connect page — additive to the local workbench.
 * Does not gate projects, sessions, tools, or local models.
 */
export function CloudLoginPage() {
  const { t, locale, setLocale } = useI18n();
  const navigate = useNavigate();
  const { cloudLinked, linking, linkError, linkCloudAccount, portalUrl } = useAccountCloud();

  const isLocalPortal = useMemo(() => {
    const url = (portalUrl ?? "").toLowerCase();
    return url.includes("127.0.0.1") || url.includes("localhost");
  }, [portalUrl]);

  useEffect(() => {
    if (!cloudLinked) return;
    void navigate({ to: "/account", replace: true });
  }, [cloudLinked, navigate]);

  useEffect(() => {
    const onLinked = () => {
      void navigate({ to: "/account", replace: true });
    };
    window.addEventListener("anycode-cloud-linked", onLinked);
    return () => window.removeEventListener("anycode-cloud-linked", onLinked);
  }, [navigate]);

  const onContinue = () => {
    void linkCloudAccount()
      .then(() => {
        void navigate({ to: "/account", replace: true });
      })
      .catch(() => undefined);
  };

  return (
    <div className="dw-cloud-gate">
      <div className="dw-cloud-gate__glow" aria-hidden />
      <div className="dw-cloud-gate__grain" aria-hidden />

      <header className="dw-cloud-gate__top">
        <BrandMark size="sm" showTitle variant="login" />
        <button
          type="button"
          className="dw-cloud-gate__locale"
          onClick={() => setLocale(locale === "zh" ? "en" : "zh")}
        >
          {locale === "zh" ? "EN" : "中文"}
        </button>
      </header>

      <main className="dw-cloud-gate__stage">
        <p className="dw-cloud-gate__eyebrow">{t("service.cloud.gateEyebrow")}</p>
        <h1 className="dw-cloud-gate__title">{t("service.cloud.gateTitle")}</h1>
        <p className="dw-cloud-gate__lede">{t("service.cloud.gateLede")}</p>

        <button
          type="button"
          className="dw-cloud-gate__cta"
          disabled={linking}
          onClick={onContinue}
        >
          <span className="dw-cloud-gate__cta-label">
            {linking ? t("service.cloud.linking") : t("service.cloud.gateCta")}
          </span>
          {!linking ? (
            <span className="dw-cloud-gate__cta-arrow" aria-hidden>
              →
            </span>
          ) : null}
        </button>

        <Link to="/conversations" className="dw-cloud-gate__offline">
          {t("service.cloud.gateBackToWorkbench")}
        </Link>

        {isLocalPortal ? (
          <p className="dw-cloud-gate__status" role="note">
            {t("service.cloud.gateLocalHint")}
          </p>
        ) : null}

        {linking ? (
          <p className="dw-cloud-gate__status" role="status">
            {t("service.cloud.gateWaiting")}
          </p>
        ) : null}

        {linkError ? (
          <p className="dw-cloud-gate__error" role="alert">
            {t("service.cloud.linkFailed")}
          </p>
        ) : null}
      </main>

      <footer className="dw-cloud-gate__foot">
        <span>{t("service.cloud.gateFoot")}</span>
      </footer>
    </div>
  );
}
