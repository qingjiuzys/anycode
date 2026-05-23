import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ServiceStatusPanel } from "@/components/ServiceStatusPanel";
import { DashboardPreferencesForm } from "@/components/settings/DashboardPreferencesForm";
import { ServiceRuntimePanel } from "@/components/settings/ServiceRuntimePanel";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";
import { useT } from "@/i18n/context";

export function SettingsServiceSection() {
  const t = useT();
  const services = useQuery({ queryKey: ["services"], queryFn: api.services });
  const runtime = useRuntimeSettings();
  const serviceStatus = useQuery({
    queryKey: ["service-status"],
    queryFn: api.serviceStatus,
    refetchInterval: 10_000,
  });
  const rt = runtime.data?.runtime;

  return (
    <>
      <DashboardPreferencesForm />
      <ServiceStatusPanel service={serviceStatus.data?.service} />
      <ServiceRuntimePanel runtime={rt} />
      <SectionCard title={t("settings.services")} noPadding>
        <div className="overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("common.name")}</th>
                <th>{t("common.address")}</th>
                <th>{t("common.status")}</th>
              </tr>
            </thead>
            <tbody>
              {(services.data?.services ?? []).map((s) => (
                <tr key={`${s.name}-${s.port}`}>
                  <td>{s.name}</td>
                  <td className="font-code text-xs">
                    {s.host}:{s.port}
                  </td>
                  <td>
                    <StatusBadge status={s.status} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </SectionCard>
    </>
  );
}
