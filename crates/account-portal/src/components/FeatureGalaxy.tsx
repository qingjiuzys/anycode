import { useEffect, useId, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useT } from "../i18n/context";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { useAuth } from "../hooks/useAuth";

export type GalaxyFeatureKey =
  | "agent"
  | "models"
  | "cloud"
  | "tools"
  | "approval"
  | "rag"
  | "speech"
  | "vision"
  | "secureMedia"
  | "desktop"
  | "automation"
  | "security";

type PlanetSpec = {
  key: GalaxyFeatureKey;
  code: string;
  tone: "cyan" | "lime" | "coral" | "violet";
  size: "sm" | "md" | "lg";
  x: number;
  y: number;
  orbit: 1 | 2 | 3;
};

const HOME_PLANETS: PlanetSpec[] = [
  { key: "agent", code: "01", tone: "cyan", size: "lg", x: 22, y: 28, orbit: 1 },
  { key: "models", code: "02", tone: "violet", size: "md", x: 72, y: 22, orbit: 1 },
  { key: "cloud", code: "03", tone: "lime", size: "md", x: 84, y: 52, orbit: 2 },
  { key: "speech", code: "04", tone: "coral", size: "sm", x: 68, y: 78, orbit: 2 },
  { key: "vision", code: "05", tone: "cyan", size: "md", x: 32, y: 76, orbit: 3 },
  { key: "secureMedia", code: "06", tone: "violet", size: "sm", x: 14, y: 52, orbit: 3 },
];

const ALL_PLANETS: PlanetSpec[] = [
  ...HOME_PLANETS,
  { key: "tools", code: "07", tone: "lime", size: "sm", x: 48, y: 16, orbit: 1 },
  { key: "approval", code: "08", tone: "coral", size: "sm", x: 90, y: 34, orbit: 2 },
  { key: "rag", code: "09", tone: "cyan", size: "sm", x: 58, y: 88, orbit: 3 },
  { key: "desktop", code: "10", tone: "violet", size: "md", x: 8, y: 34, orbit: 2 },
  { key: "automation", code: "11", tone: "lime", size: "sm", x: 42, y: 42, orbit: 1 },
  { key: "security", code: "12", tone: "coral", size: "sm", x: 78, y: 64, orbit: 3 },
];

const DUST = Array.from({ length: 48 }, (_, i) => ({
  id: i,
  left: `${(i * 37) % 97}%`,
  top: `${(i * 53) % 93}%`,
  delay: `${(i % 9) * 0.28}s`,
  size: i % 5 === 0 ? 3 : 2,
}));

type Props = {
  mode?: "home" | "page";
  title?: string;
  subtitle?: string;
};

export function FeatureGalaxy({ mode = "home", title, subtitle }: Props) {
  const t = useT();
  const { authenticated } = useAuth();
  const planets = mode === "page" ? ALL_PLANETS : HOME_PLANETS;
  const [active, setActive] = useState<GalaxyFeatureKey | null>(
    mode === "page" ? "agent" : null,
  );
  const [hovered, setHovered] = useState<GalaxyFeatureKey | null>(null);
  const titleId = useId();
  const reducedMotion = useMemo(() => {
    if (typeof window === "undefined") return false;
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }, []);

  useEffect(() => {
    if (mode === "page" || !active) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setActive(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [active, mode]);

  const activePlanet = planets.find((p) => p.key === active) ?? null;
  const focusKey = hovered ?? active;
  const heading = title ?? (mode === "page" ? t("featuresPage.title") : t("features.title"));
  const lead = subtitle ?? (mode === "page" ? t("featuresPage.subtitle") : t("features.subtitle"));
  const isPage = mode === "page";

  const map = (
    <div
      className={`nx-universe__map${reducedMotion ? " is-static" : ""}`}
      role="list"
      aria-label={t("features.eyebrow")}
    >
      <div className="nx-universe__orbits" aria-hidden>
        <i className="nx-universe__ring nx-universe__ring--1" />
        <i className="nx-universe__ring nx-universe__ring--2" />
        <i className="nx-universe__ring nx-universe__ring--3" />
        <i className="nx-universe__halo" />
      </div>

      <div className="nx-universe__core">
        <LogoMark />
        <span>anyCode</span>
      </div>

      <svg className="nx-universe__lines" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden>
        {planets.map((p) => (
          <line
            key={p.key}
            className={`nx-universe__line nx-universe__line--${p.tone}${focusKey === p.key ? " is-lit" : ""}`}
            x1="50"
            y1="50"
            x2={p.x}
            y2={p.y}
          />
        ))}
      </svg>

      {planets.map((p) => (
        <button
          key={p.key}
          type="button"
          role="listitem"
          className={`nx-universe__star nx-universe__planet nx-universe__star--${p.size} nx-universe__star--${p.tone}${active === p.key ? " is-active" : ""}${hovered === p.key ? " is-hover" : ""}`}
          style={{ left: `${p.x}%`, top: `${p.y}%` }}
          aria-pressed={active === p.key}
          aria-label={t(`features.${p.key}.title`)}
          onClick={() => setActive(p.key)}
          onMouseEnter={() => setHovered(p.key)}
          onMouseLeave={() => setHovered(null)}
          onFocus={() => setHovered(p.key)}
          onBlur={() => setHovered(null)}
        >
          <span className="nx-universe__star-orbit" aria-hidden />
          <span className="nx-universe__planet-body" aria-hidden>
            <span className="nx-universe__planet-shine" />
            <span className="nx-universe__planet-ring" />
          </span>
          <span className="nx-universe__star-label is-visible">
            <small>{p.code}</small>
            <strong>{t(`features.${p.key}.tag`)}</strong>
          </span>
        </button>
      ))}
    </div>
  );

  return (
    <section
      className={`nx-universe${mode === "home" ? " nx-universe--embed" : " nx-universe--page"}`}
      aria-labelledby={titleId}
    >
      <div className="nx-universe__sky" aria-hidden>
        {DUST.map((d) => (
          <span
            key={d.id}
            className="nx-universe__dust"
            style={{
              left: d.left,
              top: d.top,
              width: d.size,
              height: d.size,
              animationDelay: d.delay,
            }}
          />
        ))}
      </div>

      <div className={`nx-frame nx-universe__frame${isPage ? " nx-universe__frame--page" : ""}`}>
        <header className={`nx-universe__head${isPage ? " nx-page-hero" : ""}`}>
          <p className="nx-kicker">
            {mode === "home" ? `${t("features.eyebrow")} / 01` : t("featuresPage.kicker")}
          </p>
          {isPage ? (
            <h1 id={titleId}>{heading}</h1>
          ) : (
            <h2 id={titleId}>{heading}</h2>
          )}
          <p className={isPage ? "nx-page-hero__lead" : "nx-universe__lead"}>{lead}</p>
          {!isPage ? (
            <p className="nx-universe__hint">
              {t("featuresPage.galaxyHint")}
              {" · "}
              <Link className="nx-capabilities__more" to="/features">
                {t("featuresPage.explore")} <span aria-hidden>→</span>
              </Link>
            </p>
          ) : (
            <p className="nx-universe__hint">{t("featuresPage.galaxyHint")}</p>
          )}
        </header>

        {isPage ? (
          <div className="nx-universe__stage">
            {map}
            {activePlanet ? (
              <aside
                className={`nx-universe__detail nx-universe__detail--${activePlanet.tone}`}
                aria-live="polite"
              >
                <p className="nx-kicker">
                  {activePlanet.code} / {t(`features.${activePlanet.key}.tag`)}
                </p>
                <h2>{t(`features.${activePlanet.key}.title`)}</h2>
                <p>{t(`features.${activePlanet.key}.body`)}</p>
              </aside>
            ) : null}
          </div>
        ) : (
          map
        )}

        <div className="nx-universe__grid" role="list">
          {planets.map((p) => (
            <button
              key={p.key}
              type="button"
              role="listitem"
              className={`nx-universe__chip${active === p.key ? " is-active" : ""}`}
              onClick={() => setActive(p.key)}
            >
              <strong>{p.code}</strong>
              <span>{t(`features.${p.key}.title`)}</span>
            </button>
          ))}
        </div>

        {isPage ? (
          <div className="nx-universe__actions">
            <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
              {t("hero.ctaDownload")} <span aria-hidden>↓</span>
            </a>
            <Link className="nx-btn nx-btn--secondary" to={authenticated ? "/console" : "/register"}>
              {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}{" "}
              <span aria-hidden>→</span>
            </Link>
            <Link className="nx-text-link" to="/">
              {t("featuresPage.backHome")}
            </Link>
          </div>
        ) : null}
      </div>

      {!isPage && activePlanet ? (
        <div
          className="nx-universe__overlay"
          role="dialog"
          aria-modal="true"
          aria-labelledby={`${titleId}-card`}
          onClick={() => setActive(null)}
        >
          <div
            className={`nx-universe__card nx-universe__card--${activePlanet.tone}`}
            onClick={(e) => e.stopPropagation()}
          >
            <button
              type="button"
              className="nx-universe__card-close"
              aria-label={t("featuresPage.close")}
              onClick={() => setActive(null)}
            >
              ×
            </button>
            <p className="nx-kicker">
              {activePlanet.code} / {t(`features.${activePlanet.key}.tag`)}
            </p>
            <h2 id={`${titleId}-card`}>{t(`features.${activePlanet.key}.title`)}</h2>
            <p>{t(`features.${activePlanet.key}.body`)}</p>
            <span className="nx-universe__card-glow" aria-hidden />
          </div>
        </div>
      ) : null}
    </section>
  );
}

function LogoMark() {
  return (
    <svg className="nx-universe__core-mark" viewBox="0 0 32 32" aria-hidden>
      <circle cx="16" cy="16" r="14" fill="none" stroke="currentColor" strokeWidth="1.2" opacity="0.45" />
      <circle cx="16" cy="16" r="5" fill="currentColor" />
      <circle cx="16" cy="16" r="9" fill="none" stroke="currentColor" strokeWidth="0.8" opacity="0.35" />
    </svg>
  );
}
