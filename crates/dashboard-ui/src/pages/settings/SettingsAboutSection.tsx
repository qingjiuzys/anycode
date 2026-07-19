import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useT } from "@/i18n/context";
import { legalUrls } from "@anycode/site-urls";

export function SettingsAboutSection() {
  const t = useT();

  return (
    <section className="dw-settings-section space-y-4">
      <div>
        <h2 className="text-base font-semibold m-0 text-on-surface">{t("settings.about.title")}</h2>
        <p className="text-sm text-secondary mt-1 mb-0">{t("settings.about.subtitle")}</p>
      </div>

      <div className="rounded-xl border border-outline-variant bg-surface-container-low p-4 space-y-3">
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-secondary">
            {t("settings.about.algorithmLabel")}
          </div>
          <p className="m-0 mt-1 text-sm">{t("settings.about.algorithmName")}</p>
        </div>
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-secondary">
            {t("settings.about.providerLabel")}
          </div>
          <p className="m-0 mt-1 text-sm">{t("settings.about.providerName")}</p>
        </div>
        <div>
          <div className="text-xs font-semibold uppercase tracking-wide text-secondary">
            {t("settings.about.filingLabel")}
          </div>
          <p className="m-0 mt-1 text-sm">{t("settings.about.filingStatus")}</p>
        </div>
        <p className="m-0 text-sm text-secondary">{t("settings.about.aiNotice")}</p>
      </div>

      <div className="flex flex-wrap gap-2">
        <ExternalNavLink href={legalUrls.userAgreement()} className="dw-btn-secondary no-underline">
          {t("settings.about.termsLink")}
        </ExternalNavLink>
        <ExternalNavLink href={legalUrls.privacy()} className="dw-btn-secondary no-underline">
          {t("settings.about.privacyLink")}
        </ExternalNavLink>
      </div>
    </section>
  );
}
