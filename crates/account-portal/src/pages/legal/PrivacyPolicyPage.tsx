import { LegalPageLayout } from "../../components/LegalPageLayout";
import { useT } from "../../i18n/context";

export function PrivacyPolicyPage() {
  const t = useT();

  return (
    <LegalPageLayout title={t("legal.privacyTitle")} subtitle={t("legal.privacySubtitle")}>
      <section>
        <h2>{t("legal.privacyCollectTitle")}</h2>
        <p>{t("legal.privacyCollect")}</p>
      </section>
      <section>
        <h2>{t("legal.privacyUseTitle")}</h2>
        <p>{t("legal.privacyUse")}</p>
      </section>
      <section>
        <h2>{t("legal.privacyModelTitle")}</h2>
        <p>{t("legal.privacyModel")}</p>
      </section>
      <section>
        <h2>{t("legal.privacyRightsTitle")}</h2>
        <p>{t("legal.privacyRights")}</p>
      </section>
      <section>
        <h2>{t("legal.privacyContactTitle")}</h2>
        <p>{t("legal.privacyContact")}</p>
      </section>
    </LegalPageLayout>
  );
}
