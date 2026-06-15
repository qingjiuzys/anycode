import { Link } from "@tanstack/react-router";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function UpgradePromptCard() {
  const t = useT();

  return (
    <SectionCard title={t("service.usage.upgradeTitle")} className="border-warn/30">
      <p className="text-sm text-secondary m-0 mb-3">{t("service.usage.upgradeBody")}</p>
      <div className="flex flex-wrap gap-2">
        <Link to="/account" search={{ section: "plan" }} className="dw-btn-primary no-underline text-sm">
          {t("service.usage.upgradeCta")}
        </Link>
        <Link to="/account" search={{ section: "plan" }} className="dw-btn-secondary no-underline text-sm">
          {t("service.usage.comparePlans")}
        </Link>
      </div>
    </SectionCard>
  );
}

export function ServiceMockBanner() {
  const t = useT();
  return (
    <div className="dw-alert-warn text-sm" role="status">
      {t("service.mockBanner")}
    </div>
  );
}
