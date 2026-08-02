import { Link } from "react-router-dom";
import { useEffect, useState } from "react";
import { useAuth } from "../hooks/useAuth";
import { useT } from "../i18n/context";
import { LANDING_ASSETS } from "../lib/landingAssets";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";

export function HeroShowcase() {
  const { authenticated } = useAuth();
  const t = useT();
  const [loadVideo, setLoadVideo] = useState(false);

  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection
      ?.saveData;
    if (reduced || saveData) return;
    const id = window.requestAnimationFrame(() => setLoadVideo(true));
    return () => window.cancelAnimationFrame(id);
  }, []);

  return (
    <section className="scene-section scene-product-hero">
      <div className="scene-product-hero__media" aria-hidden>
        {loadVideo ? (
          <video
            className="scene-product-hero__video"
            autoPlay
            muted
            loop
            playsInline
            poster={LANDING_ASSETS.heroBg}
          >
            <source src={LANDING_ASSETS.heroVideo} type="video/mp4" />
          </video>
        ) : null}
        <img
          className="scene-product-hero__bg scene-product-hero__bg--blur"
          src={LANDING_ASSETS.heroBg}
          alt=""
        />
        <img
          className="scene-product-hero__bg scene-product-hero__bg--sharp"
          src={LANDING_ASSETS.heroBg}
          alt=""
        />
        <div className="scene-product-hero__overlay" />
      </div>

      <div className="scene-product-hero__content">
        <p className="scene-product-hero__tag">{t("hero.eyebrow")}</p>
        <h1 className="scene-product-hero__title">
          {t("hero.titleLine1")}
          <br />
          {t("hero.titleLine2")}
        </h1>
        <p className="scene-product-hero__subtitle">{t("hero.subtitle")}</p>

        <div className="scene-product-hero__actions">
          {authenticated ? (
            <>
              <Link className="scene-hero-btn scene-hero-btn--primary" to="/console">
                <span className="scene-hero-btn__label">{t("hero.ctaConsole")}</span>
              </Link>
              <Link className="scene-hero-btn" to="/console/settings">
                {t("hero.ctaLinkDesktop")}
              </Link>
            </>
          ) : (
            <>
              <Link className="scene-hero-btn scene-hero-btn--primary" to="/register">
                <span className="scene-hero-btn__label">{t("hero.ctaGetStarted")}</span>
              </Link>
              <Link className="scene-hero-btn" to="/login">
                {t("hero.ctaSignIn")}
              </Link>
            </>
          )}
          <a className="scene-hero-btn" href={DESKTOP_DOWNLOAD_URL} target="_blank" rel="noreferrer">
            {t("hero.ctaDownload")}
          </a>
        </div>

        <div className="scene-product-hero__peek">
          <div className="scene-product-hero__peek-head">
            <span>{t("hero.tileGateway")}</span>
            <span className="status-pill live">{t("hero.tileGatewayStatus")}</span>
          </div>
          <div className="scene-product-hero__peek-grid">
            <div className="scene-product-hero__peek-tile">
              <span>{t("hero.tileAccount")}</span>
              <strong>{t("hero.tileAccountValue")}</strong>
            </div>
            <div className="scene-product-hero__peek-tile">
              <span>{t("hero.tilePlan")}</span>
              <strong>{t("hero.tilePlanValue")}</strong>
            </div>
            <div className="scene-product-hero__peek-tile">
              <span>{t("hero.tileUsage")}</span>
              <strong>{t("hero.tileUsageValue")}</strong>
            </div>
            <div className="scene-product-hero__peek-tile">
              <span>{t("hero.tileDevice")}</span>
              <strong>{t("hero.tileDeviceValue")}</strong>
            </div>
          </div>
          <div className="scene-product-hero__chips">
            <span className="scene-product-hero__chip">{t("hero.chipAgent")}</span>
            <span className="scene-product-hero__chip">{t("hero.chipLocalModels")}</span>
            <span className="scene-product-hero__chip">Cloud Auto</span>
            <span className="scene-product-hero__chip">Agnes Chat</span>
          </div>
        </div>
      </div>
    </section>
  );
}
