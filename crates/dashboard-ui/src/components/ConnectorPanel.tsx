import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { GitHubIssuesPanel } from "@/components/GitHubIssuesPanel";
import { Icon } from "@/components/Icon";
import { LinearIssuesPanel } from "@/components/LinearIssuesPanel";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function ConnectorPanel() {
  const t = useT();
  const queryClient = useQueryClient();
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const connectors = useQuery({
    queryKey: ["connectors"],
    queryFn: () => api.connectors(),
  });

  const setEnabled = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      api.setConnectorEnabled(id, enabled),
    onMutate: () => setActionError(null),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["connectors"] }),
    onError: (e: Error) => setActionError(e.message),
  });

  const deleteConnector = useMutation({
    mutationFn: (id: string) => api.deleteConnector(id),
    onMutate: () => setActionError(null),
    onSuccess: () => {
      setPendingDeleteId(null);
      void queryClient.invalidateQueries({ queryKey: ["connectors"] });
    },
    onError: (e: Error) => setActionError(e.message),
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

        {actionError && (
          <p className="text-sm text-error px-4 m-0 mb-2" role="alert">
            {actionError}
          </p>
        )}

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
                  <th className="text-right">{t("common.actions")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((c) => {
                  const confirming = pendingDeleteId === c.id;
                  const deleting = deleteConnector.isPending && pendingDeleteId === c.id;
                  return (
                    <tr key={c.id}>
                      <td className="font-medium">{c.name}</td>
                      <td className="font-code text-xs">{c.source_type}</td>
                      <td className="text-secondary text-xs">{c.config_summary}</td>
                      <td>
                        <StatusBadge status={c.enabled ? "ok" : "disabled"} />
                      </td>
                      <td className="text-right">
                        <div className="inline-flex items-center justify-end gap-1.5">
                          <button
                            type="button"
                            className="dw-btn-ghost text-[12px] px-2"
                            disabled={setEnabled.isPending || deleting}
                            onClick={() =>
                              setEnabled.mutate({ id: c.id, enabled: !c.enabled })
                            }
                          >
                            {c.enabled ? t("common.disable") : t("common.enable")}
                          </button>
                          {confirming ? (
                            <>
                              <button
                                type="button"
                                className="dw-btn-ghost text-[12px] text-error px-2"
                                disabled={deleting}
                                onClick={() => deleteConnector.mutate(c.id)}
                              >
                                {deleting ? t("common.loading") : t("common.confirm")}
                              </button>
                              <button
                                type="button"
                                className="dw-btn-ghost text-[12px] px-2"
                                disabled={deleting}
                                onClick={() => setPendingDeleteId(null)}
                              >
                                {t("common.cancel")}
                              </button>
                            </>
                          ) : (
                            <button
                              type="button"
                              className="dw-btn-ghost p-1 text-error"
                              disabled={deleting}
                              onClick={() => setPendingDeleteId(c.id)}
                              aria-label={t("common.delete")}
                              title={t("common.delete")}
                            >
                              <Icon name="delete" size={16} />
                            </button>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })}
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
