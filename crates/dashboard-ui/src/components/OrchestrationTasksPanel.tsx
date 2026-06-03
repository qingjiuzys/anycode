import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function OrchestrationTasksPanel() {
  const t = useT();
  const q = useQuery({
    queryKey: ["orchestration-tasks"],
    queryFn: api.orchestrationTasks,
  });

  const tasks = Object.entries(q.data?.tasks ?? {});
  if (!q.isLoading && tasks.length === 0) {
    return null;
  }

  return (
    <SectionCard title={t("automations.parallelTasks")} noPadding>
      <div className="overflow-x-auto">
        <table className="dw-table">
          <thead>
            <tr>
              <th>{t("common.id")}</th>
              <th>{t("common.status")}</th>
              <th>{t("automations.commandSummary")}</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map(([id, rec]) => {
              const row = rec as { status?: string; subject?: string; description?: string };
              return (
                <tr key={id}>
                  <td>
                    <code className="font-code">{id.slice(0, 8)}…</code>
                  </td>
                  <td>{row.status ?? "—"}</td>
                  <td>{row.subject ?? row.description ?? "—"}</td>
                </tr>
              );
            })}
            {!q.isLoading && tasks.length === 0 && (
              <tr>
                <td colSpan={3} className="text-secondary text-center py-6">
                  {t("automations.noParallelTasks")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}
