import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { formatMessage, useT } from "../i18n/context";
import { SITE_PATHS, siteUrl } from "@anycode/site-urls";

type LatestManifest = {
  version?: string;
  arch?: string;
  filename?: string;
  url?: string;
  latest_url?: string;
  sha256?: string;
};

const GITHUB_RELEASE = "https://github.com/qingjiuzys/anycode/releases";

export function DownloadsPage() {
  const t = useT();
  const [latest, setLatest] = useState<LatestManifest | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void fetch("/downloads/latest.json", { cache: "no-store" })
      .then(async (res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return (await res.json()) as LatestManifest;
      })
      .then(setLatest)
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
      });
  }, []);

  const dmgUrl = latest?.latest_url || latest?.url || DESKTOP_DOWNLOAD_URL;
  const versionedUrl = latest?.url || dmgUrl;
  const version = latest?.version ?? "—";
  const sha = latest?.sha256;

  return (
    <div className="nx-site nx-site--downloads">
      <section className="nx-downloads-onepage" aria-labelledby="nx-downloads-title">
        <div className="nx-frame nx-downloads-onepage__inner">
          <header className="nx-page-hero nx-downloads-onepage__hero">
            <p className="nx-kicker">{t("downloads.eyebrow")}</p>
            <h1 id="nx-downloads-title">{t("downloads.title")}</h1>
            <p className="nx-page-hero__lead">{t("downloads.lede")}</p>
          </header>

          <div className="nx-downloads-onepage__panel">
            <div className="nx-downloads-onepage__meta">
              <span className="nx-downloads-onepage__os">macOS · Apple Silicon</span>
              <strong>{formatMessage(t("downloads.versionLabel"), { version })}</strong>
            </div>
            {error ? <p className="nx-downloads-onepage__error">{error}</p> : null}
            <a className="orbit-btn orbit-btn--primary nx-downloads-onepage__cta" href={dmgUrl}>
              {t("downloads.downloadLatest")} <span aria-hidden>↓</span>
            </a>
            {versionedUrl !== dmgUrl ? (
              <a className="nx-downloads-onepage__secondary" href={versionedUrl}>
                {formatMessage(t("downloads.downloadVersioned"), { version })}
              </a>
            ) : null}
            {sha ? (
              <p className="nx-downloads-onepage__sha">
                SHA-256 <code>{sha}</code>
              </p>
            ) : null}
            <nav className="nx-downloads-onepage__links" aria-label={t("downloads.otherChannels")}>
              <a href="/downloads/SHA256SUMS.txt">{t("downloads.checksums")}</a>
              <a href={GITHUB_RELEASE} target="_blank" rel="noreferrer">
                GitHub Releases
              </a>
              <Link to={SITE_PATHS.changelog}>{t("changelog.title")}</Link>
              <a href={siteUrl("docs")}>{t("downloads.installDocs")}</a>
            </nav>
          </div>
        </div>
      </section>
    </div>
  );
}
