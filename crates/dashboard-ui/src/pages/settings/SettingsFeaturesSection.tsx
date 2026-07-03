import { useEffect, useState } from "react";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import {
  FEATURE_FLAGS_EVENT,
  isReportsNavHidden,
  setReportsNavHidden,
} from "@/lib/featureFlags";

export function SettingsFeaturesSection() {
  const t = useT();
  const [hideReports, setHideReports] = useState(isReportsNavHidden);

  useEffect(() => {
    const sync = () => setHideReports(isReportsNavHidden());
    window.addEventListener(FEATURE_FLAGS_EVENT, sync);
    window.addEventListener("storage", sync);
    return () => {
      window.removeEventListener(FEATURE_FLAGS_EVENT, sync);
      window.removeEventListener("storage", sync);
    };
  }, []);

  return (
    <SectionCard title={t("settings.features.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.features.hint")}</p>
      <label className="inline-flex items-start gap-2 text-sm cursor-pointer">
        <input
          type="checkbox"
          className="mt-0.5"
          checked={hideReports}
          onChange={(e) => {
            setHideReports(e.target.checked);
            setReportsNavHidden(e.target.checked);
          }}
        />
        <span>
          <span className="font-medium text-on-surface block">{t("settings.features.hideReports")}</span>
          <span className="text-secondary text-xs">{t("settings.features.hideReportsHint")}</span>
        </span>
      </label>
    </SectionCard>
  );
}
