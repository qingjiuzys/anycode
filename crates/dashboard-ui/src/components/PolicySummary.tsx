import type { PolicySummary } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function PolicySummaryPanel({ policy }: { policy: PolicySummary | undefined }) {
  const t = useT();
  if (!policy) {
    return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  }

  return (
    <SectionCard title={t("settings.securityPolicy")}>
      <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0 mb-4">
        <dt className="text-secondary font-medium m-0">{t("settings.mode")}</dt>
        <dd className="m-0">{policy.mode}</dd>
        <dt className="text-secondary font-medium m-0">{t("settings.binding")}</dt>
        <dd className="m-0">
          <code className="font-code">{policy.host_binding}</code>
        </dd>
        <dt className="text-secondary font-medium m-0">{t("settings.remoteAccess")}</dt>
        <dd className="m-0">
          {policy.remote_access_allowed ? t("settings.allowed") : t("settings.forbidden")}
        </dd>
        <dt className="text-secondary font-medium m-0">{t("settings.writeOps")}</dt>
        <dd className="m-0">
          {policy.write_actions_allowed ? t("settings.allowed") : t("settings.forbidden")}
        </dd>
      </dl>
      <h4 className="text-xs font-semibold text-on-surface uppercase tracking-wide m-0 mb-1">
        {t("settings.safeActions")}
      </h4>
      <p className="text-sm text-secondary m-0 mb-3">{policy.safe_actions.join(" · ")}</p>
      <h4 className="text-xs font-semibold text-on-surface uppercase tracking-wide m-0 mb-1">
        {t("settings.blockedActions")}
      </h4>
      <p className="text-sm text-secondary m-0">{policy.blocked_actions.join(" · ")}</p>
    </SectionCard>
  );
}
