import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { DoctorPanel } from "@/components/settings/DoctorPanel";
import { CommandList } from "@/components/ui/CommandList";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const CHECKLIST = [
  "./scripts/build-dashboard-ui.sh",
  "cargo fmt --all -- --check",
  "cargo test -p anycode-dashboard",
  "cd crates/dashboard-ui && npm run build",
  "cargo build --release -p anycode",
];

export function SettingsOpsSection() {
  const t = useT();
  const doctor = useQuery({ queryKey: ["doctor"], queryFn: api.doctor });

  return (
    <>
      <DoctorPanel doctor={doctor.data?.doctor} />
      <SectionCard title={t("settings.releaseChecklist")}>
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.maintainerChecklistHint")}</p>
        <CommandList commands={CHECKLIST} />
      </SectionCard>
    </>
  );
}
