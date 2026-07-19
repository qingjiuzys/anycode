import { LegalPageLayout } from "../../components/LegalPageLayout";
import { useT } from "../../i18n/context";

export function UserAgreementPage() {
  const t = useT();

  return (
    <LegalPageLayout title={t("legal.userAgreementTitle")} subtitle={t("legal.userAgreementSubtitle")}>
      <section>
        <h2>{t("legal.uaScopeTitle")}</h2>
        <p>{t("legal.uaScope")}</p>
      </section>
      <section>
        <h2>{t("legal.uaAlgorithmTitle")}</h2>
        <p>{t("legal.uaAlgorithm")}</p>
      </section>
      <section>
        <h2>{t("legal.uaAiContentTitle")}</h2>
        <p>{t("legal.uaAiContent")}</p>
      </section>
      <section>
        <h2>{t("legal.uaDataTitle")}</h2>
        <p>{t("legal.uaData")}</p>
      </section>
      <section>
        <h2>{t("legal.uaContactTitle")}</h2>
        <p>{t("legal.uaContact")}</p>
      </section>
    </LegalPageLayout>
  );
}
