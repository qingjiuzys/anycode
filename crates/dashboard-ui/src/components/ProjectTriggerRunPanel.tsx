import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { api } from "@/api/client";
import type { TriggerRunResult } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function ProjectTriggerRunPanel({ projectId }: { projectId: string }) {
  const t = useT();
  const queryClient = useQueryClient();
  const [prompt, setPrompt] = useState("");
  const [goal, setGoal] = useState("");
  const [kind, setKind] = useState<"run" | "goal">("run");
  const [agent, setAgent] = useState("");
  const [lastTrigger, setLastTrigger] = useState<TriggerRunResult | null>(null);

  const recent = useQuery({
    queryKey: ["project-triggers", projectId],
    queryFn: () => api.listProjectTriggers(projectId),
    staleTime: 30_000,
  });

  const trigger = useMutation({
    mutationFn: () =>
      api.triggerProjectRun(projectId, {
        prompt: prompt.trim(),
        kind,
        goal: kind === "goal" ? goal.trim() : undefined,
        agent: agent.trim() || undefined,
      }),
    onSuccess: (data) => {
      setLastTrigger(data.trigger);
      queryClient.invalidateQueries({ queryKey: ["project-triggers", projectId] });
      queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      queryClient.invalidateQueries({ queryKey: ["running-sessions"] });
    },
  });

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const msg =
      kind === "goal"
        ? t("projectDetail.triggerConfirmGoal")
        : t("projectDetail.triggerConfirmRun");
    if (!window.confirm(msg)) return;
    trigger.mutate();
  }

  return (
    <SectionCard title={t("projectDetail.triggerRun")}>
      <p className="text-xs text-secondary m-0 mb-3">{t("projectDetail.triggerRunHint")}</p>
      <form className="space-y-3" onSubmit={onSubmit}>
        <div className="flex flex-wrap gap-2">
          <label className="flex items-center gap-1 text-sm">
            <input
              type="radio"
              name="kind"
              checked={kind === "run"}
              onChange={() => setKind("run")}
            />
            {t("projectDetail.triggerKindRun")}
          </label>
          <label className="flex items-center gap-1 text-sm">
            <input
              type="radio"
              name="kind"
              checked={kind === "goal"}
              onChange={() => setKind("goal")}
            />
            {t("projectDetail.triggerKindGoal")}
          </label>
        </div>
        {kind === "goal" && (
          <input
            className="dw-input w-full"
            placeholder={t("projectDetail.triggerGoalPlaceholder")}
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
          />
        )}
        <textarea
          className="dw-input w-full min-h-[80px]"
          placeholder={t("projectDetail.triggerPromptPlaceholder")}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          required
        />
        <input
          className="dw-input w-full max-w-xs"
          placeholder={t("projectDetail.triggerAgentPlaceholder")}
          value={agent}
          onChange={(e) => setAgent(e.target.value)}
        />
        <button
          type="submit"
          className="dw-btn-primary"
          disabled={trigger.isPending || !prompt.trim() || (kind === "goal" && !goal.trim())}
        >
          {trigger.isPending ? t("projectDetail.triggerStarting") : t("projectDetail.triggerStart")}
        </button>
      </form>

      {trigger.isError && (
        <p className="text-sm text-error mt-2 m-0">{(trigger.error as Error).message}</p>
      )}

      {lastTrigger && (
        <div className="mt-3 p-3 rounded-md bg-surface-container-low text-sm">
          <div className="font-medium">{t("projectDetail.triggerStarted")}</div>
          <div className="text-secondary font-code text-xs mt-1 break-all">
            pid={lastTrigger.pid} · {lastTrigger.command_preview}
          </div>
          <Link to="/conversations" className="text-primary text-xs hover:underline">
            {t("projectDetail.triggerWatchSessions")}
          </Link>
        </div>
      )}

      {(recent.data?.triggers ?? []).length > 0 && (
        <div className="mt-4">
          <div className="text-xs text-secondary mb-2">{t("projectDetail.triggerRecent")}</div>
          <ul className="m-0 pl-5 text-xs text-secondary space-y-1 font-code">
            {(recent.data?.triggers ?? []).slice(0, 5).map((tr) => (
              <li key={tr.trigger_id}>
                {tr.started_at} · {tr.kind} · pid {tr.pid}
              </li>
            ))}
          </ul>
        </div>
      )}
    </SectionCard>
  );
}
