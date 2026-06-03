import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ModelProfile } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

type AgentRow = {
  agent: string;
  provider: string;
  model: string;
};

function agentsToRows(agents?: Record<string, ModelProfile> | null): AgentRow[] {
  if (!agents) return [];
  return Object.entries(agents).map(([agent, profile]) => ({
    agent,
    provider: profile.provider ?? "",
    model: profile.model ?? "",
  }));
}

function rowsToAgents(rows: AgentRow[]): Record<string, ModelProfile> {
  const out: Record<string, ModelProfile> = {};
  for (const row of rows) {
    const agent = row.agent.trim();
    if (!agent) continue;
    out[agent] = {
      provider: row.provider.trim() || undefined,
      model: row.model.trim() || undefined,
    };
  }
  return out;
}

export function RoutingAgentsEditor() {
  const t = useT();
  const qc = useQueryClient();

  const catalog = useQuery({
    queryKey: ["model-catalog"],
    queryFn: () => api.modelCatalog(),
  });

  const llm = useQuery({
    queryKey: ["llm-config"],
    queryFn: () => api.getLlmConfig(),
  });

  const [rows, setRows] = useState<AgentRow[]>([]);
  const [initialAgents, setInitialAgents] = useState<string[]>([]);

  useEffect(() => {
    setRows(agentsToRows(llm.data?.routing_agents));
    setInitialAgents(Object.keys(llm.data?.routing_agents ?? {}));
  }, [llm.data?.routing_agents]);

  const save = useMutation({
    mutationFn: () => {
      const current = new Set(Object.keys(rowsToAgents(rows)));
      const deleted = initialAgents.filter((a) => !current.has(a));
      return api.putLlmConfig({
        routing_agents: rowsToAgents(rows),
        routing_agents_delete: deleted.length > 0 ? deleted : undefined,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["llm-config"] });
      qc.invalidateQueries({ queryKey: ["runtime-settings"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
      setInitialAgents(Object.keys(rowsToAgents(rows)));
    },
  });

  const addPreset = (agentId: string) => {
    if (rows.some((r) => r.agent === agentId)) return;
    setRows((prev) => [...prev, { agent: agentId, provider: "", model: "" }]);
  };

  const updateRow = (index: number, patch: Partial<AgentRow>) => {
    setRows((prev) => prev.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  };

  const removeRow = (index: number) => {
    setRows((prev) => prev.filter((_, i) => i !== index));
  };

  const providers = catalog.data?.providers ?? [];
  const presets = catalog.data?.routing_agent_presets ?? [];

  return (
    <SectionCard title={t("settings.model.routingTitle")} noPadding>
      <div className="px-4 pt-4 pb-3 border-b border-outline-variant">
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.model.routingHint")}</p>
        {presets.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-2">
            {presets.map((p) => (
              <button
                key={p.id}
                type="button"
                className="dw-chip"
                title={p.hint}
                onClick={() => addPreset(p.id)}
              >
                + {p.id}
              </button>
            ))}
          </div>
        )}
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          onClick={() => setRows((prev) => [...prev, { agent: "", provider: "", model: "" }])}
        >
          {t("settings.model.addAgent")}
        </button>
      </div>

      {rows.length === 0 ? (
        <p className="text-sm text-secondary px-4 py-4 m-0">{t("settings.runtime.noRouting")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("agents.agentCol")}</th>
                <th>{t("settings.runtime.provider")}</th>
                <th>{t("settings.runtime.modelCol")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {rows.map((row, index) => (
                <tr key={`${row.agent}-${index}`}>
                  <td>
                    <input
                      className="dw-input font-code text-xs w-full min-w-[8rem]"
                      value={row.agent}
                      onChange={(e) => updateRow(index, { agent: e.target.value })}
                      placeholder="planner"
                    />
                  </td>
                  <td>
                    <select
                      className="dw-input font-code text-xs w-full min-w-[8rem]"
                      value={row.provider}
                      onChange={(e) => updateRow(index, { provider: e.target.value })}
                    >
                      <option value="">{t("settings.model.inheritGlobal")}</option>
                      {providers.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    <input
                      className="dw-input font-code text-xs w-full min-w-[8rem]"
                      value={row.model}
                      onChange={(e) => updateRow(index, { model: e.target.value })}
                    />
                  </td>
                  <td className="text-right whitespace-nowrap">
                    <button
                      type="button"
                      className="dw-btn-ghost text-xs text-error"
                      onClick={() => removeRow(index)}
                    >
                      {t("common.delete")}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="px-4 py-4 border-t border-outline-variant">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={save.isPending || llm.isLoading}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.loading") : t("settings.model.saveRouting")}
        </button>
        {save.isSuccess && (
          <p className="text-sm text-secondary mt-2 m-0">{t("settings.model.saved")}</p>
        )}
        {save.isError && (
          <div className="dw-alert-error mt-2">{(save.error as Error).message}</div>
        )}
      </div>
    </SectionCard>
  );
}
