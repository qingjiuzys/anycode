import { Link } from "react-router-dom";
import type { ReactNode } from "react";
import { Logo } from "./Logo";
import { useT } from "../i18n/context";

export function AuthLayout({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle: string;
  children: ReactNode;
}) {
  const t = useT();

  return (
    <div className="nx-site nx-site--auth">
      <span className="nx-auth-axis nx-auth-axis--x" aria-hidden />
      <span className="nx-auth-axis nx-auth-axis--y" aria-hidden />
      <div className="nx-frame nx-site__auth-grid">
        <div className="nx-site__intro">
          <Link className="nx-site__back" to="/">
            <span aria-hidden>←</span> {t("auth.backHome")}
          </Link>
          <p className="nx-kicker">ACCOUNT / LOCAL AGENT</p>
          <h1 className="nx-site__title">{t("auth.asideTitle")}</h1>
          <p className="nx-site__lead">{t("auth.asideLead")}</p>
          <div className="nx-auth-route" aria-hidden>
            <div className="is-complete"><span>01</span><strong>ACCOUNT</strong></div>
            <i />
            <div className="is-active"><span>02</span><strong>DEVICE LINK</strong></div>
            <i />
            <div><span>03</span><strong>LOCAL AGENT</strong></div>
          </div>
          <ul className="nx-site__bullets">
            <li>{t("auth.asideItem1")}</li>
            <li>{t("auth.asideItem2")}</li>
            <li>{t("auth.asideItem3")}</li>
          </ul>
        </div>

        <div className="nx-panel nx-panel--card nx-auth-card">
          <div className="nx-auth-card__status">
            <span><i aria-hidden /> {t("console.connected")}</span>
            <span>TLS / LOCAL DEVICE</span>
          </div>
          <div className="nx-panel__head">
            <Logo size="sm" />
            <div>
              <h2>{title}</h2>
              <p className="nx-muted">{subtitle}</p>
            </div>
          </div>
          {children}
        </div>
      </div>
    </div>
  );
}
