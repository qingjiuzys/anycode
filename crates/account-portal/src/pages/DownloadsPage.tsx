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
    <div className="site-page downloads-page">
      <header className="site-page__header">
        <p className="eyebrow">{t("downloads.eyebrow")}</p>
        <h1>{t("downloads.title")}</h1>
        <p className="lede">{t("downloads.lede")}</p>
      </header>

      <section className="card downloads-card">
        <h2>{t("downloads.macosTitle")}</h2>
        <p className="muted">
          {formatMessage(t("downloads.versionLabel"), { version })}
        </p>
        {error && <p className="form-note">{error}</p>}
        <div className="downloads-actions">
          <a className="btn btn-primary" href={dmgUrl}>
            {t("downloads.downloadLatest")}
          </a>
          {versionedUrl !== dmgUrl && (
            <a className="btn btn-secondary" href={versionedUrl}>
              {formatMessage(t("downloads.downloadVersioned"), { version })}
            </a>
          )}
        </div>
        {sha && (
          <p className="muted downloads-sha">
            SHA-256: <code>{sha}</code>
          </p>
        )}
        <p className="muted">
          <a href="/downloads/SHA256SUMS.txt">{t("downloads.checksums")}</a>
        </p>
      </section>

      <section className="card downloads-card">
        <h2>{t("downloads.otherChannels")}</h2>
        <ul className="downloads-list">
          <li>
            <a href={dmgUrl}>{SITE_ORIGIN}/downloads</a>
            <span className="muted"> — {t("downloads.channelPrimary")}</span>
          </li>
          <li>
            <a href={githubRelease} target="_blank" rel="noreferrer">
              GitHub Releases
            </a>
            <span className="muted"> — {t("downloads.channelGithub")}</span>
          </li>
          <li>
            <a href={siteUrl("docs")}>{t("downloads.installDocs")}</a>
          </li>
        </ul>
      </section>
    </div>
  );
}
