import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useT } from "@/i18n/context";
import { compareSemver, fetchDesktopLatest } from "@/lib/desktopVersion";
import { legalUrls, SITE_ORIGIN } from "@anycode/site-urls";

export function SettingsAboutSection() {
  const t = useT();

  const health = useQuery({
    queryKey: ["health"],
    queryFn: api.health,
    staleTime: 60_000,
  });

  const localVersion = health.data?.version ?? "—";
  const portalOrigin =
    health.data?.account_portal_url?.replace(/\/$/, "") || SITE_ORIGIN;

  const latest = useQuery({
    queryKey: ["desktop-latest", portalOrigin],
    queryFn: () => fetchDesktopLatest(portalOrigin),
    enabled: Boolean(health.data),
    staleTime: 5 * 60_000,
    retry: 1,
  });

  const updateAvailable =
    latest.data?.version &&
    localVersion !== "—" &&
    compareSemver(latest.data.version, localVersion) > 0;

  const downloadUrl =
    latest.data?.latest_url ||
    latest.data?.url ||
    `${portalOrigin}/downloads/anyCode_latest_aarch64.dmg`;

  return (
    <section className="dw-settings-section space-y-4">
      <div>
        <h2 className="text-base font-semibold m-0 text-on-surface">{t("settings.about.title")}</h2>
        <p className="text-sm text-secondary mt-1 mb-0">{t("settings.about.subtitle")}</p>
      </div>

      <div className="rounded-xl border border-outline-variant bg-surface-container-low p-4 space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <div className="text-xs font-semibold uppercase tracking-wide text-secondary">
              {t("settings.about.versionLabel")}
            </div>
            <p className="m-0 mt-1 text-sm font-code">{localVersion}</p>
          </div>
          <button
            type="button"
            className="dw-btn-secondary text-xs"
            disabled={latest.isFetching}
            onClick={() => void latest.refetch()}
          >
            {latest.isFetching
              ? t("settings.about.checkingUpdate")
              : t("settings.about.checkUpdate")}
          </button>
        </div>

        {latest.isError && (
          <p className="m-0 text-sm text-error">
            {(latest.error as Error).message || t("settings.about.updateCheckFailed")}
          </p>
        )}

        {updateAvailable ? (
          <div className="rounded-lg border border-primary/30 bg-primary/5 px-3 py-2 space-y-2">
            <p className="m-0 text-sm">
              {t("settings.about.updateAvailable").replace(
                "{version}",
                latest.data!.version,
              )}
            </p>
            <a
              href={downloadUrl}
              className="dw-btn-primary text-xs no-underline inline-flex"
              target="_blank"
              rel="noreferrer"
            >
              {t("settings.about.downloadUpdate")}
            </a>
          </div>
        ) : latest.isSuccess && !latest.isFetching ? (
          <p className="m-0 text-sm text-secondary">{t("settings.about.upToDate")}</p>
        ) : null}
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
