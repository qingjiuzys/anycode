import type { RuntimeSettings } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function RuntimeConfigPanel({ runtime }: { runtime?: RuntimeSettings }) {
  const t = useT();
  if (!runtime) return null;

  return (
    <>
      <SectionCard title={t("settings.runtime.configPath")}>
        {!runtime.config_present && (
          <p className="text-sm text-secondary m-0 mb-3">{t("settings.runtime.configMissing")}</p>
        )}
        <p className="text-sm font-code text-secondary m-0 mb-4 break-all">{runtime.config_path}</p>
        <dl className="grid grid-cols-[minmax(6rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
          <dt className="text-secondary font-medium m-0">{t("settings.runtime.globalProvider")}</dt>
          <dd className="m-0 font-code">{runtime.global_provider ?? "—"}</dd>
          <dt className="text-secondary font-medium m-0">{t("settings.runtime.globalModel")}</dt>
          <dd className="m-0 font-code">{runtime.global_model ?? "—"}</dd>
        </dl>
        <p className="text-xs text-secondary mt-4 m-0">{t("settings.runtime.modelHint")}</p>
      </SectionCard>

      <SectionCard title={t("settings.runtime.routingAgents")} noPadding>
        {runtime.routing_agents.length === 0 ? (
          <p className="text-sm text-secondary px-4 py-4 m-0">{t("settings.runtime.noRouting")}</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("agents.agentCol")}</th>
                  <th>{t("settings.runtime.provider")}</th>
                  <th>{t("settings.runtime.modelCol")}</th>
                </tr>
              </thead>
              <tbody>
                {runtime.routing_agents.map((a) => (
                  <tr key={a.agent}>
                    <td className="font-code text-xs">{a.agent}</td>
                    <td className="text-secondary">{a.provider ?? "—"}</td>
                    <td className="text-secondary font-code text-xs">{a.model ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SectionCard>

      {runtime.model_routes && Object.keys(runtime.model_routes).length > 0 && (
        <SectionCard title={t("settings.runtime.modelRoutes")}>
          <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-48 whitespace-pre-wrap m-0">
            {JSON.stringify(runtime.model_routes, null, 2)}
          </pre>
        </SectionCard>
      )}
    </>
  );
}
