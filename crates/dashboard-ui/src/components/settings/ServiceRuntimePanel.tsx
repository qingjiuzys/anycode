import type { RuntimeSettings } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function ServiceRuntimePanel({ runtime }: { runtime?: RuntimeSettings }) {
  const t = useT();
  if (!runtime) return null;

  return (
    <SectionCard title={t("settings.tabs.service")}>
      <dl className="grid grid-cols-[minmax(6rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0 mb-4">
        <dt className="text-secondary font-medium m-0">{t("settings.runtime.dashboardHost")}</dt>
        <dd className="m-0 font-code">{runtime.host}</dd>
        <dt className="text-secondary font-medium m-0">{t("settings.runtime.dashboardPort")}</dt>
        <dd className="m-0 font-code">{runtime.port}</dd>
        <dt className="text-secondary font-medium m-0">{t("settings.runtime.sseGlobal")}</dt>
        <dd className="m-0 font-code text-xs">{runtime.sse_events_path}</dd>
        <dt className="text-secondary font-medium m-0">{t("settings.runtime.sseProject")}</dt>
        <dd className="m-0 font-code text-xs">{runtime.sse_project_events_path}</dd>
      </dl>
      <p className="text-xs text-secondary m-0">{t("settings.runtime.cliHint")}</p>
    </SectionCard>
  );
}
