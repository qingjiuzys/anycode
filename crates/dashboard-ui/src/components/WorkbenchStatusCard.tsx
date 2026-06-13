import type { BootstrapSummary } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const PHASE_LABELS: Record<string, string> = {
  v2_complete: "V1+V2 complete",
  v3_week1: "V3 Week 1 — cost & KPI",
  v3_week2: "V3 Week 2 — connectors & gate history",
  v3_week3: "V3 Week 3 — control plane POC",
  v3_week4: "V3 Week 4 — session usage & gate streaming",
  v3_week5: "V3 Week 5 — security inbox & token charts",
  v3_week6: "V3 Week 6 — live CLI cooperative cancel",
  v3_week7: "V3 Week 7 — UI trigger run (sandbox)",
  v3_week8: "V3 Week 8 — interactive Web tool approval",
  v3_week9: "V3 Week 9 — session inbox & approval badges",
  v3_week10: "V3 Week 10 — Conversations approval workflow",
};

/** Workbench ship phase + planning doc links on Home. */
export function WorkbenchStatusCard({ bootstrap }: { bootstrap?: BootstrapSummary }) {
  const t = useT();
  if (!bootstrap?.workbench_phase) return null;

  const phase = bootstrap.workbench_phase;
  const label = PHASE_LABELS[phase] ?? phase;

  return (
    <SectionCard title={t("home.workbenchStatus")}>
      <p className="text-sm m-0 mb-2">
        <span className="font-medium text-primary">{label}</span>
      </p>
      <p className="text-sm text-secondary m-0 mb-2">{t("home.workbenchStatusBody")}</p>
      <ul className="m-0 pl-5 text-sm space-y-1 font-code text-secondary">
        <li>docs/workbench/digital-workbench-STATUS.md</li>
        <li>{bootstrap.planning_doc}</li>
        <li>docs/workbench/digital-workbench-deploy-production.md</li>
      </ul>
    </SectionCard>
  );
}
