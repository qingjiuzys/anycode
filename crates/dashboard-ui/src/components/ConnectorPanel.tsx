import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { GitHubIssuesPanel } from "@/components/GitHubIssuesPanel";
import { LinearIssuesPanel } from "@/components/LinearIssuesPanel";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function ConnectorPanel() {
  const t = useT();

  const connectors = useQuery({
    queryKey: ["connectors"],
    queryFn: () => api.connectors(),
  });

  const rows = connectors.data?.connectors ?? [];
  const githubConnectors = rows.filter(
    (c) => c.enabled && c.source_type === "github" && c.config_summary,
  );
  const linearConnectors = rows.filter(
    (c) => c.enabled && c.source_type === "linear" && c.config_summary,
  );

  return (
    <>
      <SectionCard title={t("settings.connectors")} noPadding>
        <div className="px-4 pt-4 pb-3">
          <div className="dw-alert-warn mb-3 text-sm">{t("settings.connectorReadOnly")}</div>
          <p className="text-sm text-secondary m-0">{t("settings.connectorReadOnlyDetail")}</p>
        </div>

        {rows.length === 0 ? (
          <p className="text-sm text-secondary px-4 pb-4 m-0">{t("settings.noConnectors")}</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("common.name")}</th>
                  <th>{t("conversations.type")}</th>
                  <th>{t("settings.connectorSummary")}</th>
                  <th>{t("common.status")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((c) => (
                  <tr key={c.id}>
                    <td className="font-medium">{c.name}</td>
                    <td className="font-code text-xs">{c.source_type}</td>
                    <td className="text-secondary text-xs">{c.config_summary}</td>
                    <td>
                      <StatusBadge status={c.enabled ? "ok" : "disabled"} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SectionCard>

      {githubConnectors.map((c) => (
        <GitHubIssuesPanel
          key={c.id}
          connectorId={c.id}
          connectorName={c.name}
          repo={c.config_summary}
        />
      ))}
      {linearConnectors.map((c) => (
        <LinearIssuesPanel
          key={c.id}
          connectorId={c.id}
          connectorName={c.name}
          team={c.config_summary}
        />
      ))}
    </>
  );
}
