import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { formatMessage, useT } from "../i18n/context";
import { SITE_PATHS, siteUrl } from "@anycode/site-urls";

type PlatformId = "macos-aarch64" | "macos-x86_64" | "windows-x64";

type Artifact = {
  platform: PlatformId | string;
  version: string;
  arch?: string;
  filename: string;
  url: string;
  latest_url?: string;
  sha256?: string;
  ext?: string;
  latest?: boolean;
};

type PlatformInfo = {
  version: string;
  arch?: string;
  filename: string;
  url: string;
  latest_url?: string;
  sha256?: string;
  ext?: string;
};

type ReleasesManifest = {
  generated_at?: string;
  latest_by_platform?: Record<string, string>;
  platforms?: Record<string, PlatformInfo>;
  artifacts?: Artifact[];
};

type LatestManifest = {
  version?: string;
  arch?: string;
  filename?: string;
  url?: string;
  latest_url?: string;
  sha256?: string;
  platforms?: Record<string, PlatformInfo>;
};

const PLATFORM_ORDER: PlatformId[] = ["macos-aarch64", "macos-x86_64", "windows-x64"];

const GITHUB_RELEASE = "https://github.com/qingjiuzys/anycode/releases";

function platformLabelKey(id: PlatformId): "downloads.platformMacArm" | "downloads.platformMacIntel" | "downloads.platformWindows" {
  switch (id) {
    case "macos-aarch64":
      return "downloads.platformMacArm";
    case "macos-x86_64":
      return "downloads.platformMacIntel";
    case "windows-x64":
      return "downloads.platformWindows";
  }
}

export function DownloadsPage() {
  const t = useT();
  const [releases, setReleases] = useState<ReleasesManifest | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const relRes = await fetch("/downloads/releases.json", { cache: "no-store" });
        if (relRes.ok) {
          const data = (await relRes.json()) as ReleasesManifest;
          if (!cancelled) {
            setReleases(data);
            setError(null);
          }
          return;
        }
        const latRes = await fetch("/downloads/latest.json", { cache: "no-store" });
        if (!latRes.ok) throw new Error(`HTTP ${latRes.status}`);
        const latest = (await latRes.json()) as LatestManifest;
        if (cancelled) return;
        const platforms = latest.platforms ?? {};
        if (!platforms["macos-aarch64"] && latest.url) {
          platforms["macos-aarch64"] = {
            version: latest.version ?? "—",
            arch: latest.arch,
            filename: latest.filename ?? "anyCode_latest_aarch64.dmg",
            url: latest.url,
            latest_url: latest.latest_url,
            sha256: latest.sha256,
            ext: "dmg",
          };
        }
        setReleases({
          latest_by_platform: Object.fromEntries(
            Object.entries(platforms).map(([k, v]) => [k, v.version]),
          ),
          platforms,
          artifacts: Object.entries(platforms).map(([platform, info]) => ({
            platform,
            ...info,
            latest: true,
          })),
        });
        setError(null);
      } catch (e: unknown) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : String(e));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const cards = useMemo(() => {
    return PLATFORM_ORDER.map((id) => {
      const info = releases?.platforms?.[id];
      const history = (releases?.artifacts ?? [])
        .filter((a) => a.platform === id)
        .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }));
      return { id, info, history };
    });
  }, [releases]);

  return (
    <div className="nx-site nx-site--downloads">
      <section className="nx-downloads-onepage" aria-labelledby="nx-downloads-title">
        <div className="nx-frame nx-downloads-onepage__inner">
          <header className="nx-page-hero nx-downloads-onepage__hero">
            <p className="nx-kicker">{t("downloads.eyebrow")}</p>
            <h1 id="nx-downloads-title">{t("downloads.title")}</h1>
            <p className="nx-page-hero__lead">{t("downloads.lede")}</p>
          </header>

          {error ? <p className="nx-downloads-onepage__error">{error}</p> : null}

          <div className="nx-downloads-grid">
            {cards.map(({ id, info, history }) => {
              const available = Boolean(info?.url || info?.latest_url);
              const href = info?.latest_url || info?.url || (id === "macos-aarch64" ? DESKTOP_DOWNLOAD_URL : undefined);
              const version = info?.version ?? "—";
              return (
                <article key={id} className="nx-downloads-card">
                  <div className="nx-downloads-card__meta">
                    <span className="nx-downloads-card__os">{t(platformLabelKey(id))}</span>
                    <strong>
                      {available
                        ? formatMessage(t("downloads.versionLabel"), { version })
                        : t("downloads.comingSoon")}
                    </strong>
                  </div>
                  {available && href ? (
                    <a className="orbit-btn orbit-btn--primary nx-downloads-card__cta" href={href}>
                      {t("downloads.downloadLatest")} <span aria-hidden>↓</span>
                    </a>
                  ) : (
                    <button type="button" className="orbit-btn orbit-btn--primary nx-downloads-card__cta" disabled>
                      {t("downloads.comingSoon")}
                    </button>
                  )}
                  {info?.sha256 ? (
                    <p className="nx-downloads-card__sha">
                      SHA-256 <code>{info.sha256}</code>
                    </p>
                  ) : null}
                  {history.length > 1 ? (
                    <details className="nx-downloads-card__history">
                      <summary>{t("downloads.previousVersions")}</summary>
                      <ul>
                        {history.map((art) => (
                          <li key={`${art.platform}-${art.version}-${art.filename}`}>
                            <a href={art.url}>
                              {formatMessage(t("downloads.downloadVersioned"), {
                                version: art.version,
                              })}
                            </a>
                          </li>
                        ))}
                      </ul>
                    </details>
                  ) : null}
                </article>
              );
            })}
          </div>

          <nav className="nx-downloads-onepage__links" aria-label={t("downloads.otherChannels")}>
            <a href="/downloads/SHA256SUMS.txt">{t("downloads.checksums")}</a>
            <a href="/downloads/releases.json">{t("downloads.manifest")}</a>
            <a href={GITHUB_RELEASE} target="_blank" rel="noreferrer">
              GitHub Releases
            </a>
            <Link to={SITE_PATHS.changelog}>{t("changelog.title")}</Link>
            <a href={siteUrl("docs")}>{t("downloads.installDocs")}</a>
          </nav>
        </div>
      </section>
    </div>
  );
}
