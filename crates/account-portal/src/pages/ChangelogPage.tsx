import { Link } from "react-router-dom";
import {
  CHANGELOG_RELEASES,
  type ChangelogRelease,
  type ChangelogSectionKind,
  type LocalizedText,
} from "../data/changelog";
import { useLocale, useT } from "../i18n/context";
import { SITE_GITHUB, SITE_PATHS } from "@anycode/site-urls";

function pick(text: LocalizedText, locale: "zh" | "en"): string {
  return locale === "zh" ? text.zh : text.en;
}

function sectionLabel(kind: ChangelogSectionKind, t: (key: string) => string): string {
  switch (kind) {
    case "added":
      return t("changelog.sectionAdded");
    case "changed":
      return t("changelog.sectionChanged");
    case "fixed":
      return t("changelog.sectionFixed");
  }
}

function ReleaseBlock({
  release,
  locale,
  isLatest,
  t,
}: {
  release: ChangelogRelease;
  locale: "zh" | "en";
  isLatest: boolean;
  t: (key: string) => string;
}) {
  const releaseUrl = `${SITE_GITHUB}/releases/tag/${release.tag}`;

  return (
    <article className="nx-changelog__entry">
      <div className="nx-changelog__rail" aria-hidden>
        <span className={`nx-changelog__dot${isLatest ? " nx-changelog__dot--latest" : ""}`} />
      </div>
      <div className="nx-changelog__body">
        <header className="nx-changelog__header">
          <div className="nx-changelog__meta">
            <h2 className="nx-changelog__version">v{release.version}</h2>
            {isLatest ? <span className="nx-changelog__badge">{t("changelog.latestBadge")}</span> : null}
          </div>
          <time className="nx-changelog__date" dateTime={release.date}>
            {release.date}
          </time>
        </header>

        {release.summary ? (
          <p className="nx-changelog__summary">{pick(release.summary, locale)}</p>
        ) : null}

        {release.sections.map((section) => (
          <section key={section.kind} className="nx-changelog__section">
            <h3 className={`nx-changelog__section-title nx-changelog__section-title--${section.kind}`}>
              {sectionLabel(section.kind, t)}
            </h3>
            <ul>
              {section.items.map((item, idx) => (
                <li key={idx}>{pick(item, locale)}</li>
              ))}
            </ul>
          </section>
        ))}

        <footer className="nx-changelog__footer">
          <a className="nx-text-link" href={releaseUrl} target="_blank" rel="noreferrer">
            {t("changelog.viewOnGithub")}
          </a>
        </footer>
      </div>
    </article>
  );
}

export function ChangelogPage() {
  const t = useT();
  const locale = useLocale();

  return (
    <div className="nx-site nx-site--changelog">
      <section className="nx-changelog">
        <div className="nx-changelog__frame">
          <header className="nx-page-hero nx-changelog__hero">
            <p className="nx-kicker">{t("changelog.eyebrow")}</p>
            <h1>{t("changelog.title")}</h1>
            <p className="nx-page-hero__lead">{t("changelog.lede")}</p>
            <div className="nx-changelog__hero-links">
              <Link className="nx-btn nx-btn--primary" to={SITE_PATHS.downloads}>
                {t("changelog.downloadCta")}
              </Link>
              <a className="nx-btn nx-btn--ghost" href={`${SITE_GITHUB}/releases`} target="_blank" rel="noreferrer">
                GitHub Releases
              </a>
            </div>
          </header>

          <ol className="nx-changelog__timeline" aria-label={t("changelog.timelineAria")}>
            {CHANGELOG_RELEASES.map((release, index) => (
              <li key={release.version}>
                <ReleaseBlock
                  release={release}
                  locale={locale}
                  isLatest={index === 0}
                  t={t}
                />
              </li>
            ))}
          </ol>
        </div>
      </section>
    </div>
  );
}
