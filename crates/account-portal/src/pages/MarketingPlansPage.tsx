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
        <div className="nx-frame">
          <header className="nx-section-head">
            <div>
              <p className="nx-kicker">{t("plans.eyebrow")}</p>
              <h1>{t("plans.title")}</h1>
            </div>
            <p>{LEAD[locale]}</p>
          </header>

          {loading ? <p className="nx-muted">{t("common.loading")}</p> : null}

          <div className="nx-plan-grid">
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

          <p className="nx-plans__status">{t("hero.reviewStatus")}</p>

          <div className="nx-universe__actions" style={{ marginTop: 24 }}>
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
