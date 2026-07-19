import { Link } from "react-router-dom";
import { useAuth } from "../hooks/useAuth";
import { useT } from "../i18n/context";

export function FinalCta() {
  const { authenticated } = useAuth();
  const t = useT();

  return (
    <section className="scene-section scene-section--auto scene-cta-banner">
      <div className="scene-cta-banner__mesh" aria-hidden />
      <div className="scene-cta-banner__inner">
        <h2>{t("cta.title")}</h2>
        <div className="scene-cta-banner__actions">
          {authenticated ? (
            <Link className="scene-hero-btn scene-hero-btn--primary" to="/console/settings">
              <span className="scene-hero-btn__label">{t("cta.openDesktop")}</span>
            </Link>
          ) : (
            <>
              <Link className="scene-hero-btn scene-hero-btn--primary" to="/register">
                <span className="scene-hero-btn__label">{t("cta.register")}</span>
              </Link>
              <Link className="scene-hero-btn" to="/login">
                {t("cta.signInExisting")}
              </Link>
            </>
          )}
        </div>
      </div>
    </section>
  );
}
