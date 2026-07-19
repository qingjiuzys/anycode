import { Link } from "react-router-dom";
import type { ReactNode } from "react";
import { useT } from "../i18n/context";

export function LegalPageLayout({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
}) {
  const t = useT();

  return (
    <div className="nx-site nx-site--legal">
      <div className="nx-frame nx-site__legal">
        <Link className="nx-site__back" to="/">
          <span aria-hidden>←</span> {t("auth.backHome")}
        </Link>
        <article className="nx-panel nx-panel--legal">
          <p className="nx-kicker">LEGAL / COMPLIANCE</p>
          <h1>{title}</h1>
          {subtitle ? <p className="nx-muted nx-panel__subtitle">{subtitle}</p> : null}
          <div className="nx-panel__body legal-page__body">{children}</div>
        </article>
      </div>
    </div>
  );
}
