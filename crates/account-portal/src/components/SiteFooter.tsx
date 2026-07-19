import { Link } from "react-router-dom";
import { Logo } from "./Logo";
import { ALGORITHM_DISCLOSURE_PUBLIC } from "../lib/compliance";
import { SITE_PATHS } from "@anycode/site-urls";
import { useT } from "../i18n/context";

export function SiteFooter() {
  const t = useT();

  return (
    <footer className="nx-footer nx-site-footer">
      <div className="nx-frame nx-footer__inner nx-site-footer__inner">
        <div className="nx-footer__brand">
          <Logo size="sm" />
          <strong>{t("common.brand")}</strong>
        </div>
        <nav className="nx-site-footer__links" aria-label={t("footer.colLegal")}>
          {ALGORITHM_DISCLOSURE_PUBLIC ? (
            <Link to={SITE_PATHS.legalAlgorithmDisclosure}>{t("legal.algorithmLink")}</Link>
          ) : null}
          <Link to={SITE_PATHS.legalUserAgreement}>{t("legal.userAgreementLink")}</Link>
          <Link to={SITE_PATHS.legalPrivacy}>{t("legal.privacyLink")}</Link>
        </nav>
        <span>{t("hero.reviewStatus")}</span>
      </div>
    </footer>
  );
}
