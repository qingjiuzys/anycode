import type { DeliveryReadiness } from "@/api/types";
import { Link } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { SectionCard } from "./ui/SectionCard";
import { useT } from "@/i18n/context";

export function DeliveryReadinessCard({
  readiness,
  compact,
}: {
  readiness?: DeliveryReadiness;
  compact?: boolean;
}) {
  const t = useT();
  if (!readiness) return null;

  const hasIssues =
    readiness.blocked_sessions > 0 ||
    readiness.failed_required_gates > 0 ||
    readiness.unverified_artifacts > 0;

  if (!hasIssues && readiness.status === "ok") return null;

  return (
    <SectionCard
      title={t("home.deliveryReadiness")}
      action={
        !compact ? (
          <Link to="/assets" className="text-xs text-primary hover:underline">
            {t("panels.viewDetails")} →
          </Link>
        ) : undefined
      }
    >
      <div className={`grid grid-cols-1 md:grid-cols-3 gap-4 ${compact ? "" : ""}`}>
        <Metric
          icon="warning"
          iconClass="text-warn"
          label={t("home.blockedSessions")}
          value={readiness.blocked_sessions}
          warn={readiness.blocked_sessions > 0}
        />
        <Metric
          icon="cancel"
          iconClass="text-error"
          label={t("home.failedGates")}
          value={readiness.failed_required_gates}
          error={readiness.failed_required_gates > 0}
        />
        <Metric
          icon="help_center"
          iconClass="text-secondary"
          label={t("home.unverifiedAssets")}
          value={readiness.unverified_artifacts}
        />
      </div>
      {!compact && readiness.projects.length > 0 && (
        <div className="mt-4 overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("assets.project")}</th>
                <th>{t("home.readinessScore")}</th>
                <th>{t("home.blocked")}</th>
              </tr>
            </thead>
            <tbody>
              {readiness.projects.slice(0, 8).map((p) => (
                <tr key={p.project_id}>
                  <td>
                    <Link to="/projects/$projectId" params={{ projectId: p.project_id }}>
                      {p.project_name}
                    </Link>
                  </td>
                  <td>{p.readiness_score}</td>
                  <td>{p.blocked_sessions}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </SectionCard>
  );
}

function Metric({
  icon,
  iconClass,
  label,
  value,
  warn,
  error,
}: {
  icon: string;
  iconClass: string;
  label: string;
  value: number;
  warn?: boolean;
  error?: boolean;
}) {
  const box = error
    ? "bg-error-container border-error/20"
    : warn
      ? "bg-warn-container border-warn/30"
      : "bg-surface-container-low border-outline-variant";
  return (
    <div className={`flex items-start gap-3 p-4 rounded border ${box}`}>
      <Icon name={icon} className={iconClass} size={20} />
      <div className="flex flex-col">
        <span className="text-xs font-medium text-secondary">{label}</span>
        <span className="text-lg font-semibold text-on-surface">{value}</span>
      </div>
    </div>
  );
}
