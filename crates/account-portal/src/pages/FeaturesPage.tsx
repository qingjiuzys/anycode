import { Link } from "react-router-dom";
import { useAuth } from "../hooks/useAuth";
import { useT } from "../i18n/context";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import type { GalaxyFeatureKey } from "../components/FeatureGalaxy";

const FEATURE_KEYS: GalaxyFeatureKey[] = [
  "agent",
  "models",
  "cloud",
  "tools",
  "approval",
  "rag",
  "speech",
  "vision",
  "secureMedia",
  "desktop",
  "automation",
  "security",
];

/** Capability overview — single scroll page. */
export function FeaturesPage() {
  const t = useT();
  const { authenticated } = useAuth();

  return (
    <div className="nx-site nx-site--features">
      <section className="nx-features-onepage" aria-labelledby="nx-features-title">
        <div className="nx-frame nx-features-onepage__inner">
          <header className="nx-page-hero nx-features-onepage__hero">
            <p className="nx-kicker">{t("featuresPage.kicker")}</p>
            <h1 id="nx-features-title">{t("featuresPage.title")}</h1>
            <p className="nx-page-hero__lead">{t("featuresPage.subtitle")}</p>
          </header>

          <ul className="nx-features-onepage__grid">
            {FEATURE_KEYS.map((key, index) => (
              <li key={key} className="nx-features-onepage__card">
                <div className="nx-features-onepage__card-head">
                  <span className="nx-features-onepage__code">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <span className="nx-features-onepage__tag">{t(`features.${key}.tag`)}</span>
                </div>
                <h2>{t(`features.${key}.title`)}</h2>
                <p>{t(`features.${key}.body`)}</p>
              </li>
            ))}
          </ul>

          <div className="nx-features-onepage__actions">
            <a className="orbit-btn orbit-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
              {t("hero.ctaDownload")} <span aria-hidden>↓</span>
            </a>
            <Link
              className="orbit-btn orbit-btn--ghost"
              to={authenticated ? "/console" : "/register"}
            >
              {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}{" "}
              <span aria-hidden>→</span>
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
