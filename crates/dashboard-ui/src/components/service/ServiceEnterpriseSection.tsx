import { Link } from "@tanstack/react-router";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

const CAPABILITIES = ["rbac", "audit", "sso", "teamBilling"] as const;

export function ServiceEnterpriseSection() {
  const t = useT();
  const { entitlements } = useAccountCloud();
  if (!entitlements) return null;
  const org = entitlements.organization;

  return (
    <div className="space-y-6">
      <SectionCard title={t("service.enterprise.overview")}>
        <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0 mb-4">
          <dt className="text-secondary m-0">{t("service.enterprise.orgName")}</dt>
          <dd className="m-0">{org.name}</dd>
          <dt className="text-secondary m-0">{t("service.plan.currentSubscription")}</dt>
          <dd className="m-0">{t(`service.plan.tiers.${entitlements.plan}`)}</dd>
          <dt className="text-secondary m-0">{t("service.enterprise.members")}</dt>
          <dd className="m-0">{org.members.length}</dd>
          <dt className="text-secondary m-0">{t("service.enterprise.sso")}</dt>
          <dd className="m-0">
            <StatusBadge
              status={org.ssoStatus === "configured" ? "ok" : "pending"}
              label={t(`service.enterprise.ssoStatus.${org.ssoStatus}`)}
            />
          </dd>
        </dl>
        <div className="flex flex-wrap gap-2">
          <Link to="/account" search={{ section: "plan" }} className="dw-btn-primary no-underline text-sm">
            {t("service.enterprise.requestTrial")}
          </Link>
          <a href="mailto:sales@anycode.dev" className="dw-btn-secondary no-underline text-sm">
            {t("service.enterprise.contactSales")}
          </a>
        </div>
      </SectionCard>

      <SectionCard title={t("service.enterprise.membersTitle")}>
        <div className="overflow-x-auto -mx-4 px-4">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("common.name")}</th>
                <th>{t("auth.email")}</th>
                <th>{t("auth.role")}</th>
                <th>{t("common.status")}</th>
                <th>{t("service.enterprise.lastActive")}</th>
              </tr>
            </thead>
            <tbody>
              {org.members.map((m) => (
                <tr key={m.id}>
                  <td>{m.name}</td>
                  <td className="text-secondary text-xs">{m.email}</td>
                  <td>{m.role}</td>
                  <td>
                    <StatusBadge
                      status={m.status === "active" ? "ok" : "pending"}
                      label={t(`service.enterprise.memberStatus.${m.status}`)}
                    />
                  </td>
                  <td className="text-secondary text-xs tabular-nums">{m.lastActive}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </SectionCard>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {CAPABILITIES.map((key) => (
          <SectionCard key={key} title={t(`service.enterprise.capabilities.${key}.title`)}>
            <p className="text-sm text-secondary m-0">
              {t(`service.enterprise.capabilities.${key}.desc`)}
            </p>
            {key === "audit" && (
              <Link to="/audit" className="inline-block mt-3 text-sm">
                {t("service.enterprise.viewAudit")}
              </Link>
            )}
          </SectionCard>
        ))}
      </div>
    </div>
  );
}
