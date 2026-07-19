import { Link } from "react-router-dom";
import { usePlanTiers } from "../lib/plans";
import { useT } from "../i18n/context";

export function PlanPreview() {
  const t = useT();
  const { plans } = usePlanTiers();

  return (
    <section className="scene-section scene-plans" id="plans">
      <div className="scene-plans__header">
        <p className="scene-brand-statement__tag">{t("plans.eyebrow")}</p>
        <h2>{t("plans.title")}</h2>
        <p>{t("plans.pageDesc")}</p>
      </div>
      <div className="scene-plans__grid">
        {plans.map((p) => (
          <article
            className={`scene-plans__card${p.featured ? " scene-plans__card--featured" : ""}`}
            key={p.id}
          >
            {(p.promoLabel || p.featured) && (
              <span className="scene-plans__badge">
                {p.promoLabel || t("common.recommended")}
              </span>
            )}
            <span className="scene-plans__plan-kind">
              {p.id === "free" ? t("plans.localCore") : t("plans.cloudOptional")}
            </span>
            <h3>{p.name}</h3>
            <p className="scene-plans__price">{p.price}</p>
            <p className="scene-plans__desc">{p.desc}</p>
            <ul className="scene-plans__highlights">
              {p.highlights.slice(0, 2).map((h) => (
                <li key={h}>{h}</li>
              ))}
            </ul>
            <Link
              className={`lx-btn ${p.featured ? "lx-btn--primary" : "lx-btn--ghost"}`}
              to="/console/plans"
            >
              {t("common.viewDetails")}
            </Link>
          </article>
        ))}
      </div>
      <p className="scene-plans__review-note">{t("hero.reviewStatus")}</p>
    </section>
  );
}
