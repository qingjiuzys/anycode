import { LegalPageLayout } from "../../components/LegalPageLayout";
import { ALGORITHM_DISCLOSURE_PUBLIC } from "../../lib/compliance";
import { useT } from "../../i18n/context";

export function AlgorithmDisclosurePage() {
  const t = useT();

  if (!ALGORITHM_DISCLOSURE_PUBLIC) {
    return (
      <LegalPageLayout title={t("legal.algorithmPendingTitle")}>
        <section>
          <p>{t("legal.algorithmPendingBody")}</p>
        </section>
      </LegalPageLayout>
    );
  }

  return (
    <LegalPageLayout
      title={t("legal.algorithmTitle")}
      subtitle={t("legal.algorithmSubtitle")}
    >
      <section>
        <h2>{t("legal.algorithmNameLabel")}</h2>
        <p>{t("legal.algorithmName")}</p>
      </section>
      <section>
        <h2>{t("legal.algorithmProviderLabel")}</h2>
        <p>{t("legal.algorithmProvider")}</p>
      </section>
      <section>
        <h2>{t("legal.algorithmPrincipleTitle")}</h2>
        <p>{t("legal.algorithmPrinciple")}</p>
      </section>
      <section>
        <h2>{t("legal.algorithmMechanismTitle")}</h2>
        <p>{t("legal.algorithmMechanism")}</p>
      </section>
      <section>
        <h2>{t("legal.algorithmPurposeTitle")}</h2>
        <p>{t("legal.algorithmPurpose")}</p>
      </section>
      <section>
        <h2>{t("legal.algorithmScenariosTitle")}</h2>
        <ul>
          <li>anyCode — {t("legal.scenarioAnyCode")}</li>
          <li>{t("legal.scenarioYoushu")}</li>
          <li>{t("legal.scenarioLingzhi")}</li>
          <li>{t("legal.scenarioWorkbench")}</li>
        </ul>
      </section>
      <section>
        <h2>{t("legal.algorithmNoticeTitle")}</h2>
        <p>{t("legal.algorithmNotice")}</p>
      </section>
    </LegalPageLayout>
  );
}
