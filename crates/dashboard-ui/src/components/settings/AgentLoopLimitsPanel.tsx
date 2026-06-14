import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function AgentLoopLimitsPanel() {
  const t = useT();
  const queryClient = useQueryClient();
  const limitsQ = useQuery({
    queryKey: ["agent-limits"],
    queryFn: api.agentLimits,
  });

  const [maxAgentTurns, setMaxAgentTurns] = useState(8);
  const [maxToolCalls, setMaxToolCalls] = useState(32);

  useEffect(() => {
    if (limitsQ.data) {
      setMaxAgentTurns(limitsQ.data.max_agent_turns);
      setMaxToolCalls(limitsQ.data.max_tool_calls);
    }
  }, [limitsQ.data]);

  const save = useMutation({
    mutationFn: () =>
      api.setAgentLimits({
        max_agent_turns: maxAgentTurns,
        max_tool_calls: maxToolCalls,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["agent-limits"] });
    },
  });

  const data = limitsQ.data;
  const busy = save.isPending || limitsQ.isLoading;
  const defaults = data?.defaults;
  const bounds = data?.limits;

  return (
    <SectionCard title={t("settings.agentLimits.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.agentLimits.hint")}</p>

      <div className="grid gap-4 sm:grid-cols-2 max-w-xl">
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">{t("settings.agentLimits.maxAgentTurns")}</span>
          <span className="text-xs text-secondary">
            {t("settings.agentLimits.maxAgentTurnsHint")}
          </span>
          <input
            type="number"
            className="dw-input tabular-nums"
            min={bounds?.max_agent_turns_min ?? 1}
            max={bounds?.max_agent_turns_max ?? 64}
            value={maxAgentTurns}
            disabled={busy}
            onChange={(e) => setMaxAgentTurns(Number(e.target.value))}
          />
        </label>

        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">{t("settings.agentLimits.maxToolCalls")}</span>
          <span className="text-xs text-secondary">
            {t("settings.agentLimits.maxToolCallsHint")}
          </span>
          <input
            type="number"
            className="dw-input tabular-nums"
            min={bounds?.max_tool_calls_min ?? 1}
            max={bounds?.max_tool_calls_max ?? 256}
            value={maxToolCalls}
            disabled={busy}
            onChange={(e) => setMaxToolCalls(Number(e.target.value))}
          />
        </label>
      </div>

      {defaults ? (
        <p className="text-xs text-secondary m-0 mt-3">
          {t("settings.agentLimits.defaults")
            .replace("{turns}", String(defaults.max_agent_turns))
            .replace("{tools}", String(defaults.max_tool_calls))}
        </p>
      ) : null}

      {data?.config_path ? (
        <p className="text-xs text-secondary m-0 mt-2">
          {t("settings.runtime.configPath")}:{" "}
          <code className="font-code break-all">{data.config_path}</code>
        </p>
      ) : null}

      <div className="flex flex-wrap items-center gap-3 mt-4">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={busy || !Number.isFinite(maxAgentTurns) || !Number.isFinite(maxToolCalls)}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.saving") : t("common.save")}
        </button>
        {save.isSuccess ? (
          <span className="text-xs text-success">{t("settings.agentLimits.saved")}</span>
        ) : null}
        {save.isError ? (
          <span className="text-xs text-error">{(save.error as Error).message}</span>
        ) : null}
      </div>

      <p className="text-xs text-secondary m-0 mt-3">{t("settings.agentLimits.restartHint")}</p>
    </SectionCard>
  );
}
