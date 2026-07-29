import { useEffect, useState } from "react";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { formatMessage, useT } from "../i18n/context";
import { SITE_ORIGIN, siteUrl } from "@anycode/site-urls";

type LatestManifest = {
  version?: string;
  arch?: string;
  filename?: string;
  url?: string;
  latest_url?: string;
  sha256?: string;
};

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
  const githubRelease = `https://github.com/qingjiuzys/anycode/releases`;

  return (
    <div className="nx-site nx-site--downloads">
      <section className="nx-downloads">
        <div className="nx-downloads__frame">
          <header className="nx-page-hero">
            <p className="nx-kicker">{t("downloads.eyebrow")}</p>
            <h1>{t("downloads.title")}</h1>
            <p className="nx-page-hero__lead">{t("downloads.lede")}</p>
          </header>

          <div className="nx-downloads__hero-card">
            <div className="nx-downloads__hero-meta">
              <span className="nx-downloads__os">macOS · Apple Silicon</span>
              <strong>{formatMessage(t("downloads.versionLabel"), { version })}</strong>
            </div>
            {error ? <p className="nx-downloads__error">{error}</p> : null}
            <a className="nx-btn nx-btn--primary nx-downloads__cta" href={dmgUrl}>
              {t("downloads.downloadLatest")} <span aria-hidden>↓</span>
            </a>
            {versionedUrl !== dmgUrl ? (
              <a className="nx-text-link" href={versionedUrl}>
                {formatMessage(t("downloads.downloadVersioned"), { version })}
              </a>
            ) : null}
            {sha ? (
              <p className="nx-downloads__sha">
                SHA-256 <code>{sha}</code>
              </p>
            ) : null}
            <p className="nx-downloads__checksum">
              <a href="/downloads/SHA256SUMS.txt">{t("downloads.checksums")}</a>
            </p>
          </div>

          <aside className="nx-downloads__channels">
            <h2>{t("downloads.otherChannels")}</h2>
            <ul>
              <li>
                <a href={dmgUrl}>{SITE_ORIGIN}/downloads</a>
                <span>{t("downloads.channelPrimary")}</span>
              </li>
              <li>
                <a href={githubRelease} target="_blank" rel="noreferrer">
                  GitHub Releases
                </a>
                <span>{t("downloads.channelGithub")}</span>
              </li>
              <li>
                <a href={siteUrl("docs")}>{t("downloads.installDocs")}</a>
              </li>
            </ul>
          </aside>
        </div>
      </section>
    </div>
  );
}
