import { useT } from "../i18n/context";

const FEATURE_KEYS = ["agent", "models", "cloud", "speech", "vision", "secureMedia"] as const;

const ICON_PATHS: Record<(typeof FEATURE_KEYS)[number], string[]> = {
  agent: ["M12 3 4.5 7.3v9.4L12 21l7.5-4.3V7.3L12 3Z", "M9 10h6M9 14h4"],
  models: ["M5 6.5 12 3l7 3.5-7 3.5-7-3.5Z", "M5 11.5 12 15l7-3.5M5 16.5 12 20l7-3.5"],
  cloud: ["M7 18h10a4 4 0 0 0 .6-7.95A6 6 0 0 0 6.1 8.6 4.7 4.7 0 0 0 7 18Z"],
  speech: ["M9 6a3 3 0 0 1 6 0v6a3 3 0 0 1-6 0V6Z", "M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v3"],
  vision: ["M3.5 12s3-5 8.5-5 8.5 5 8.5 5-3 5-8.5 5S3.5 12 3.5 12Z", "M12 9.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5Z"],
  secureMedia: ["M7 10V7a5 5 0 0 1 10 0v3", "M5 10h14v10H5V10Z", "M12 14v2"],
};

function FeatureIcon({ name }: { name: (typeof FEATURE_KEYS)[number] }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      {ICON_PATHS[name].map((path) => (
        <path key={path} d={path} />
      ))}
    </svg>
  );
}

export function FeatureSection() {
  const t = useT();

  return (
    <>
      <section className="scene-section scene-section--auto scene-brand-statement" id="product">
        <p className="scene-brand-statement__tag">{t("features.eyebrow")}</p>
        <h2>{t("features.title")}</h2>
        <p>{t("features.subtitle")}</p>
      </section>

      <section className="scene-section scene-features">
        <div className="scene-features__grid">
          {FEATURE_KEYS.map((key) => (
            <article className="scene-features__card" key={key}>
              <div className="scene-features__content">
                <span className="scene-features__icon">
                  <FeatureIcon name={key} />
                </span>
                <span className="scene-features__tag">{t(`features.${key}.tag`)}</span>
                <h3>{t(`features.${key}.title`)}</h3>
                <p>{t(`features.${key}.body`)}</p>
              </div>
            </article>
          ))}
        </div>
      </section>
    </>
  );
}
