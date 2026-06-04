import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { DoctorPanel } from "@/components/settings/DoctorPanel";
import { ReportPreferencesPanel } from "@/components/settings/ReportPreferencesPanel";
import { CommandList } from "@/components/ui/CommandList";
import { SectionCard } from "@/components/ui/SectionCard";
import { getDensity, setDensity, type Density } from "@/hooks/useDensity";
import { useT } from "@/i18n/context";

const CHECKLIST = [
  "./scripts/build-dashboard-ui.sh",
  "cargo fmt --all -- --check",
  "cargo test -p anycode-dashboard",
  "cd crates/dashboard-ui && npm run build",
  "cargo build --release -p anycode",
];

const DENSITY_KEYS: Record<Density, string> = {
  comfortable: "settings.densityComfortable",
  compact: "settings.densityCompact",
  audit: "settings.densityAudit",
};

export function SettingsOpsSection() {
  const t = useT();
  const doctor = useQuery({ queryKey: ["doctor"], queryFn: api.doctor });
  const density = getDensity();

  return (
    <>
      <ReportPreferencesPanel />
      <DoctorPanel doctor={doctor.data?.doctor} />
      <SectionCard title={t("settings.releaseChecklist")}>
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.maintainerChecklistHint")}</p>
        <CommandList commands={CHECKLIST} />
      </SectionCard>
      <SectionCard title={t("settings.density")}>
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
    </>
  );
}
