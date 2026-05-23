import type { ToolGovernanceResponse } from "@/api/types";
import { useT } from "@/i18n/context";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";

export function ToolGovernancePanel({
  governance,
}: {
  governance?: ToolGovernanceResponse;
}) {
  const t = useT();
  const tools = governance?.tools ?? [];
  return (
    <SectionCard title={t("settings.toolGovernance")} noPadding>
      <div className="p-4 grid grid-cols-3 gap-3">
        <Mini label={t("settings.toolsTotal")} value={governance?.summary.total ?? 0} />
        <Mini label={t("settings.highRiskTools")} value={governance?.summary.high_risk ?? 0} />
        <Mini label={t("settings.approvalGaps")} value={governance?.summary.approval_gaps ?? 0} />
      </div>
      <div className="overflow-x-auto">
        <table className="dw-table">
          <thead>
            <tr>
              <th>{t("common.name")}</th>
              <th>{t("settings.category")}</th>
              <th>{t("settings.riskTier")}</th>
              <th>{t("settings.requiresApproval")}</th>
              <th>{t("settings.auditLevel")}</th>
            </tr>
          </thead>
          <tbody>
            {tools.slice(0, 24).map((tool) => (
              <tr key={tool.id}>
                <td className="font-code text-xs">{tool.id}</td>
                <td>{tool.category}</td>
                <td>
                  <StatusBadge status={tool.risk_tier} />
                </td>
                <td>{tool.requires_approval ? t("session.yes") : t("session.no")}</td>
                <td>{tool.audit_level}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}

function Mini({ label, value }: { label: string; value: number }) {
  return (
    <div className="dw-stat-card">
      <div className="dw-stat-label">{label}</div>
      <div className="dw-stat-value text-base">{value}</div>
    </div>
  );
}
