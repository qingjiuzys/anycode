import type { ServiceStatusDetail } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { StatusBadge } from "./ui/StatusBadge";

export function ServiceStatusPanel({ service }: { service?: ServiceStatusDetail }) {
  const t = useT();
  if (!service) return null;
  return (
    <SectionCard title={t("settings.serviceStatus")}>
      <p className="text-sm m-0 mb-2">
        <strong className="text-on-surface">{service.name}</strong> @ {service.host}:{service.port}{" "}
        <StatusBadge status={service.status} />
      </p>
      <p className="text-xs text-secondary m-0 mb-1">
        v{service.version} · pid {service.pid ?? "—"} · {service.auth_mode}
      </p>
      <p className="text-xs text-secondary m-0 mb-1">
        {t("settings.startedAt")}: {service.started_at}
      </p>
      <p className="text-xs text-secondary m-0 mb-1">
        {t("settings.uiDist")}:{" "}
        {service.ui_dist_present ? service.ui_dist : t("settings.uiDistMissing")}
      </p>
      <p className="text-xs text-secondary m-0 mb-1">
        {t("settings.sseSubscribers")}: {service.sse_subscribers}
      </p>
      {service.last_event_at && (
        <p className="text-xs text-secondary m-0">
          {t("settings.lastEvent")}: {service.last_event_at}
        </p>
      )}
      {!service.loopback && (
        <div className="dw-alert-error mt-3">{t("settings.nonLoopbackWarn")}</div>
      )}
    </SectionCard>
  );
}
