import { Link } from "react-router-dom";
import { useAuth } from "../hooks/useAuth";
import { useLocale, useT } from "../i18n/context";
import { usePlanTiers } from "../lib/plans";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";

const LEAD = {
  zh: "本地能力永久免费，云端能力按需选择。",
  en: "Local capabilities stay free. Add cloud only when you need it.",
} as const;

/** Public marketing plans page (not console /console/plans). */
export function MarketingPlansPage() {
  const t = useT();
  const locale = useLocale();
  const { authenticated } = useAuth();
  const { plans, loading } = usePlanTiers();

  return (
    <div className="nx-site nx-site--plans">
      <section className="nx-plans nx-plans--page">
        <div className="nx-plans__frame">
          <header className="nx-page-hero">
            <p className="nx-kicker">{t("plans.eyebrow")}</p>
            <h1>{t("plans.title")}</h1>
            <p className="nx-page-hero__lead">{LEAD[locale]}</p>
          </header>

          {loading ? (
            <div className="nx-plan-grid nx-plan-grid--dark" aria-busy="true">
              {[0, 1, 2].map((i) => (
                <div className="nx-plan nx-plan--skeleton" key={i} />
              ))}
            </div>
          ) : (
            <div className="nx-plan-grid nx-plan-grid--dark">
              {plans.map((plan, index) => (
                <article className={`nx-plan${plan.featured ? " is-featured" : ""}`} key={plan.id}>
                  <div className="nx-plan__head">
                    <span>0{index + 1}</span>
                    {plan.promoLabel || plan.featured ? (
                      <b>{plan.promoLabel || t("common.recommended")}</b>
                    ) : null}
                  </div>
                  <h3>{plan.name}</h3>
                  <p className="nx-plan__price">{plan.price}</p>
                  <p className="nx-plan__desc">{plan.desc}</p>
                  <ul>
                    {plan.highlights.map((highlight) => (
                      <li key={highlight}>{highlight}</li>
                    ))}
                  </ul>
                  <Link
                    className="nx-plan__link"
                    to={authenticated ? "/console/plans" : "/register"}
                  >
                    {authenticated ? t("common.viewDetails") : t("nav.getStarted")}{" "}
                    <span aria-hidden>→</span>
                  </Link>
                </article>
              ))}
            </div>
          )}

          <p className="nx-plans__status">{t("hero.reviewStatus")}</p>

          <div className="nx-universe__actions">
            <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
              {t("hero.ctaDownload")} <span aria-hidden>↓</span>
            </a>
            <Link className="nx-text-link" to="/">
              {t("featuresPage.backHome")}
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
