import { Link } from "react-router-dom";
import { Logo } from "./Logo";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { ALGORITHM_DISCLOSURE_PUBLIC } from "../lib/compliance";
import { SITE_PATHS } from "@anycode/site-urls";
import { useT } from "../i18n/context";

export function LandingFooter() {
  const t = useT();

  return (
    <footer className="lx-footer scene-section--auto">
      <div className="lx-footer__inner">
        <div className="lx-footer__brand">
          <Logo size="sm" />
          <span>{t("common.brand")}</span>
        </div>
        <div className="lx-footer__columns">
          <div>
            <div className="lx-footer__col-title">{t("footer.colProduct")}</div>
            <div className="lx-footer__col-links">
              <Link to={SITE_PATHS.downloads}>{t("nav.downloads")}</Link>
              <Link to={SITE_PATHS.changelog}>{t("changelog.title")}</Link>
              <a href={DESKTOP_DOWNLOAD_URL} target="_blank" rel="noreferrer">
                {t("hero.ctaDownload")}
              </a>
              <Link to={SITE_PATHS.register}>{t("nav.getStarted")}</Link>
            </div>
          </div>
          <div>
            <div className="lx-footer__col-title">{t("footer.colAccount")}</div>
            <div className="lx-footer__col-links">
              <Link to={SITE_PATHS.login}>{t("nav.signIn")}</Link>
              <Link to="/console">{t("nav.console")}</Link>
            </div>
          </div>
          <div>
            <div className="lx-footer__col-title">{t("footer.colLegal")}</div>
            <div className="lx-footer__col-links">
              {ALGORITHM_DISCLOSURE_PUBLIC ? (
                <Link to={SITE_PATHS.legalAlgorithmDisclosure}>{t("legal.algorithmLink")}</Link>
              ) : null}
              <Link to={SITE_PATHS.legalUserAgreement}>{t("legal.userAgreementLink")}</Link>
              <Link to={SITE_PATHS.legalPrivacy}>{t("legal.privacyLink")}</Link>
            </div>
          </div>
        </div>
        <p className="lx-footer__copy">{t("footer.tagline")}</p>
      </div>
    </footer>
  );
}
