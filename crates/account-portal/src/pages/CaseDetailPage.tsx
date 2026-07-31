import { Link, Navigate, useParams } from "react-router-dom";
import { CaseSlidePreview } from "../components/CaseSlidePreview";
import { CaseThumb } from "../components/CaseThumb";
import { useLocale, useT } from "../i18n/context";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import {
  caseCopy,
  casePath,
  getCase,
  gridCases,
  type CaseItemId,
} from "../lib/cases";

export function CaseDetailPage() {
  const { caseId = "" } = useParams();
  const locale = useLocale();
  const t = useT();
  const copy = caseCopy(locale);
  const def = getCase(caseId);

  if (!def || !(caseId in copy.items)) {
    return <Navigate to="/" replace />;
  }

  const item = copy.items[caseId as CaseItemId];
  const others = gridCases().filter((c) => c.id !== def.id);
  if (!def.featured) {
    const featured = getCase("launch-ppt");
    if (featured) others.unshift(featured);
  }

  return (
    <div className="nx-site nx-site--case">
      <section className="nx-case-detail">
        <div className="nx-frame">
          <Link className="nx-site__back" to="/">
            ← {copy.backHome}
          </Link>

          <header className="nx-case-detail__head">
            <p className="nx-kicker">{item.tag}</p>
            <h1>{item.title}</h1>
            <p className="nx-case-detail__lead">{item.summary}</p>
          </header>

          <div className="nx-case-detail__grid">
            <CaseSlidePreview
              title={item.slideTitle}
              sub={item.slideSub}
              body={item.slideBody}
              strong={item.slideStrong}
              steps={item.slideSteps}
            />

            <div className="nx-case-detail__aside">
              <div className="nx-case-detail__block">
                <span>{copy.promptLabel}</span>
                <blockquote>
                  <span aria-hidden>&gt;</span>
                  <p>{item.prompt}</p>
                </blockquote>
              </div>
              <dl className="nx-case-detail__meta">
                <div>
                  <dt>{copy.modelLabel}</dt>
                  <dd>{def.model}</dd>
                </div>
                <div>
                  <dt>{copy.skillLabel}</dt>
                  <dd>{def.skill}</dd>
                </div>
                <div>
                  <dt>{copy.outputLabel}</dt>
                  <dd>{item.output}</dd>
                </div>
              </dl>
            </div>
          </div>

          <section className="nx-case-detail__steps" aria-labelledby="nx-case-steps">
            <h2 id="nx-case-steps">{copy.stepsLabel}</h2>
            <ol>
              {item.steps.map((step, i) => (
                <li key={step}>
                  <span>0{i + 1}</span>
                  <p>{step}</p>
                </li>
              ))}
            </ol>
          </section>

          <section className="nx-case-detail__try">
            <div>
              <h2>{copy.tryLabel}</h2>
              <p>{copy.tryBody}</p>
            </div>
            <div className="nx-case-detail__try-actions">
              {def.demoUrl ? (
                <a className="nx-btn nx-btn--primary" href={def.demoUrl} target="_blank" rel="noreferrer">
                  {copy.openDemo} <span aria-hidden>→</span>
                </a>
              ) : null}
              <a
                className={def.demoUrl ? "nx-btn nx-btn--ghost" : "nx-btn nx-btn--primary"}
                href={DESKTOP_DOWNLOAD_URL}
              >
                {t("hero.ctaDownload")} <span aria-hidden>↓</span>
              </a>
            </div>
          </section>

          {others.length > 0 ? (
            <section className="nx-case-detail__more" aria-label={copy.sectionTitle}>
              <h2>{copy.sectionTitle}</h2>
              <div className="nx-cases__grid nx-cases__grid--loose">
                {others.slice(0, 3).map((c, index) => {
                  const other = copy.items[c.id as CaseItemId];
                  return (
                    <Link
                      className="nx-case-card"
                      key={c.id}
                      to={casePath(c.id)}
                    >
                      <div className="nx-case-card__thumb">
                        <CaseThumb kind={c.kind} />
                      </div>
                      <span className="nx-case-card__code">0{index + 1}</span>
                      <h3>{other.title}</h3>
                      <p>{other.summary}</p>
                      <footer>
                        <span>{c.model}</span>
                        <span>{c.skill}</span>
                      </footer>
                    </Link>
                  );
                })}
              </div>
            </section>
          ) : null}
        </div>
      </section>
    </div>
  );
}
