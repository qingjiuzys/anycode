import { SectionCard } from "@/components/ui/SectionCard";
import { getDensity, setDensity, type Density } from "@/hooks/useDensity";
import { useT } from "@/i18n/context";

const DENSITY_KEYS: Record<Density, string> = {
  comfortable: "settings.densityComfortable",
  compact: "settings.densityCompact",
  audit: "settings.densityAudit",
};

export function UiDensityPanel() {
  const t = useT();
  const density = getDensity();

  return (
    <SectionCard title={t("settings.userPrefs.densityTitle")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.userPrefs.densityHint")}</p>
      <div className="flex flex-wrap gap-2">
        {(["comfortable", "compact", "audit"] as Density[]).map((d) => (
          <button
            key={d}
            type="button"
            className={`dw-chip${density === d ? " active" : ""}`}
            onClick={() => setDensity(d)}
          >
            {t(DENSITY_KEYS[d])}
          </button>
        ))}
      </div>
    </SectionCard>
  );
}
