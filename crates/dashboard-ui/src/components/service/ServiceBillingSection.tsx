import type { ServiceEntitlements } from "@/api/types/service";
import { PLAN_CATALOG } from "@/lib/planCatalog";
import { EmptyState } from "@/components/EmptyState";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceBillingSection() {
  const t = useT();
  const { entitlements, updateBillingContact } = useAccountCloud();

  if (!entitlements) return null;
  const catalog = PLAN_CATALOG[entitlements.plan];
  const estimatedAmount =
    entitlements.billingCycle === "yearly"
      ? catalog.yearlyPriceUsd
      : catalog.monthlyPriceUsd;

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <SectionCard title={t("service.billing.currentPeriod")}>
          <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
            <dt className="text-secondary m-0">{t("service.plan.currentSubscription")}</dt>
            <dd className="m-0 font-medium">{t(`service.plan.tiers.${entitlements.plan}`)}</dd>
            <dt className="text-secondary m-0">{t("service.billing.period")}</dt>
            <dd className="m-0 tabular-nums">
              {entitlements.billingPeriod.start} — {entitlements.billingPeriod.end}
            </dd>
            <dt className="text-secondary m-0">{t("service.billing.estimatedAmount")}</dt>
            <dd className="m-0 tabular-nums">${estimatedAmount.toFixed(2)}</dd>
            <dt className="text-secondary m-0">{t("common.status")}</dt>
            <dd className="m-0">
              <StatusBadge
                status={
                  entitlements.subscriptionStatus === "active" ||
                  entitlements.subscriptionStatus === "trialing"
                    ? "ok"
                    : "warn"
                }
                label={t(`service.status.${entitlements.subscriptionStatus}`)}
              />
            </dd>
          </dl>
        </SectionCard>

        <SectionCard title={t("service.billing.paymentMethod")}>
          {entitlements.paymentMethodBound ? (
            <p className="text-sm m-0">{t("service.billing.paymentBound")}</p>
          ) : (
            <EmptyState
              compact
              icon="article"
              title={t("service.billing.noPaymentMethod")}
              description={t("service.billing.paymentComingSoon")}
            />
          )}
        </SectionCard>
      </div>

      <SectionCard title={t("service.billing.invoices")}>
        <div className="overflow-x-auto -mx-4 px-4">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("service.billing.invoiceNumber")}</th>
                <th>{t("service.billing.period")}</th>
                <th>{t("service.billing.amount")}</th>
                <th>{t("common.status")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {entitlements.invoices.map((inv) => (
                <tr key={inv.id}>
                  <td className="font-code text-xs">{inv.number}</td>
                  <td className="text-secondary text-xs tabular-nums">
                    {inv.periodStart} — {inv.periodEnd}
                  </td>
                  <td className="tabular-nums">${inv.amountUsd.toFixed(2)}</td>
                  <td>
                    <StatusBadge
                      status={inv.status === "paid" ? "ok" : inv.status === "pending" ? "pending" : "warn"}
                      label={t(`service.billing.invoiceStatus.${inv.status}`)}
                    />
                  </td>
                  <td>
                    <button type="button" className="dw-btn-ghost text-xs" disabled>
                      {t("service.billing.download")}
                    </button>
                  </td>
                </tr>
              ))}
              {entitlements.invoices.length === 0 && (
                <tr>
                  <td colSpan={5} className="text-center text-secondary py-6">
                    {t("service.billing.noInvoices")}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </SectionCard>

      <BillingContactForm
        contact={entitlements.billingContact}
        onChange={(patch) => void updateBillingContact(patch)}
      />
    </div>
  );
}

function BillingContactForm({
  contact,
  onChange,
}: {
  contact: ServiceEntitlements["billingContact"];
  onChange: (patch: Partial<ServiceEntitlements["billingContact"]>) => void;
}) {
  const t = useT();

  return (
    <SectionCard title={t("service.billing.contactTitle")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("service.billing.contactHint")}</p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary">{t("auth.email")}</span>
          <input
            className="dw-input"
            type="email"
            value={contact.email}
            onChange={(e) => onChange({ email: e.target.value })}
            placeholder={t("service.billing.emailPlaceholder")}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary">{t("service.billing.companyName")}</span>
          <input
            className="dw-input"
            value={contact.companyName}
            onChange={(e) => onChange({ companyName: e.target.value })}
            placeholder={t("service.billing.companyPlaceholder")}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm md:col-span-2">
          <span className="text-secondary">{t("service.billing.taxId")}</span>
          <input
            className="dw-input"
            value={contact.taxId}
            onChange={(e) => onChange({ taxId: e.target.value })}
            placeholder={t("service.billing.taxPlaceholder")}
          />
        </label>
      </div>
    </SectionCard>
  );
}
