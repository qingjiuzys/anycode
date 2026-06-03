import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

type Props = {
  sessionId: string;
  live?: boolean;
};

export function SessionBackgroundTasksPanel({ sessionId, live }: Props) {
  const t = useT();
  const q = useQuery({
    queryKey: ["session-background-tasks", sessionId],
    queryFn: () => api.sessionBackgroundTasks(sessionId),
    refetchInterval: live ? 3_000 : false,
  });

  const orch = q.data?.orchestration_tasks ?? [];
  const tools = q.data?.agent_tool_calls ?? [];
  const empty = !q.isLoading && orch.length === 0 && tools.length === 0;

  return (
    <SectionCard title={t("session.backgroundTasks")} noPadding>
      <p className="text-sm text-secondary px-4 pt-3 m-0">{t("session.backgroundTasksHint")}</p>
      <div className="overflow-x-auto mt-2">
        <table className="dw-table">
          <thead>
            <tr>
              <th>{t("common.name")}</th>
              <th>{t("common.status")}</th>
              <th>{t("automations.commandSummary")}</th>
              <th>{t("common.time")}</th>
            </tr>
          </thead>
          <tbody>
            {orch.map((row) => (
              <tr key={`orch-${row.id}`}>
                <td>{t("session.orchestrationTask")}</td>
                <td>{row.status ?? "—"}</td>
                <td>
                  <code className="font-code text-xs">{row.id.slice(0, 8)}…</code>
                  {" · "}
                  {row.subject ?? row.description ?? "—"}
                </td>
                <td>—</td>
              </tr>
            ))}
            {tools.map((row, idx) => (
              <tr key={`tool-${row.occurred_at}-${idx}`}>
                <td>{t("session.agentToolCall")}</td>
                <td>{row.severity}</td>
                <td>
                  {row.tool}: {row.title}
                  {row.body ? ` — ${row.body}` : ""}
                </td>
                <td className="text-xs whitespace-nowrap">{row.occurred_at}</td>
              </tr>
            ))}
            {empty && (
              <tr>
                <td colSpan={4} className="text-secondary text-center py-6">
                  {t("session.noBackgroundTasks")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}
