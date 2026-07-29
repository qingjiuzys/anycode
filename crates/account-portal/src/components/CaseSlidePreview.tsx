type Props = {
  title: string;
  sub: string;
  body: string;
  strong: string;
  steps: readonly string[];
  fileLabel?: string;
};

/** FDE-editorial mini slide used on home feature + case detail. */
export function CaseSlidePreview({
  title,
  sub,
  body,
  strong,
  steps,
  fileLabel = "slides / 01-cover.html",
}: Props) {
  return (
    <div className="nx-case-slide">
      <div className="nx-case-slide__chrome">
        <span />
        <span />
        <span />
        <b>{fileLabel}</b>
      </div>
      <div className="nx-case-slide__stage">
        <div className="nx-fde-slide">
          <div className="nx-fde-slide__label">
            <em>01</em>
            EDITORIAL · PPT
          </div>
          <h3>
            {title}
            <br />
            <span>{sub}</span>
          </h3>
          <p>
            {body}
            <strong>{strong}</strong>
          </p>
          <div className="nx-fde-slide__ladder">
            {steps.map((step, i) => (
              <div key={step} className={i === steps.length - 1 ? "is-hot" : undefined}>
                <small>0{i + 1}</small>
                <b>{step}</b>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
