import type { ReactNode } from "react";

export function ConsolePage({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="console-page nx-console-page">
      <header className="console-header nx-console-page__header">
        <div>
          <p className="nx-kicker">ACCOUNT CONTROL / LIVE</p>
          <h1>{title}</h1>
          {description && <p className="muted">{description}</p>}
        </div>
        <div className="nx-console-page__status" aria-label="Cloud link online">
          <span aria-hidden />
          CLOUD LINK
          <strong>ONLINE</strong>
        </div>
      </header>
      <div className="nx-console-page__content">{children}</div>
    </div>
  );
}
