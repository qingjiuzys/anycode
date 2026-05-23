import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { AgentRoleCards } from "@/components/AgentRoleCards";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function AgentsPage() {
  const t = useT();
  const queryClient = useQueryClient();
  const stats = useQuery({
    queryKey: ["agent-stats"],
    queryFn: () => api.agentStats(30),
  });
  const skills = useQuery({
    queryKey: ["skills"],
    queryFn: () => api.skills(80),
  });
  const rescan = useMutation({
    mutationFn: api.rescanSkills,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
    },
  });

  return (
    <>
      <PageHeader
        title={t("agents.title")}
        subtitle={t("agents.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("agents.title") },
        ]}
      />

      <AgentRoleCards agents={stats.data?.agents ?? []} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SectionCard title={t("agents.agentStats")} noPadding>
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("agents.agentCol")}</th>
                  <th>{t("agents.model")}</th>
                  <th className="text-right">{t("agents.sessionCount")}</th>
                  <th>{t("agents.recent")}</th>
                </tr>
              </thead>
              <tbody>
                {(stats.data?.agents ?? []).map((a) => (
                  <tr key={`${a.agent_type}-${a.model}`}>
                    <td>
                      <strong>{a.agent_type || "—"}</strong>
                    </td>
                    <td className="text-secondary font-code text-xs">{a.model || "—"}</td>
                    <td className="text-right">{a.sessions_count}</td>
                    <td className="text-secondary text-xs">{a.last_started_at ?? "—"}</td>
                  </tr>
                ))}
                {!stats.isLoading && (stats.data?.agents ?? []).length === 0 && (
                  <tr>
                    <td colSpan={4} className="text-secondary text-center py-6">
                      {t("agents.emptyUsage")}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </SectionCard>

        <SectionCard
          title={t("agents.skills")}
          action={
            <button
              type="button"
              className="dw-btn-secondary"
              disabled={rescan.isPending}
              onClick={() => rescan.mutate()}
            >
              <Icon name="refresh" size={16} />
              {rescan.isPending ? t("agents.rescanning") : t("agents.rescan")}
            </button>
          }
          noPadding
        >
          {rescan.isSuccess && (
            <p className="text-sm text-secondary px-4 pt-4 m-0">
              {t("agents.rescanSuccess").replace("{n}", String(rescan.data.skills_synced))}
            </p>
          )}
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("common.id")}</th>
                  <th>{t("common.name")}</th>
                  <th className="text-right">{t("agents.projectsCount")}</th>
                </tr>
              </thead>
              <tbody>
                {(skills.data?.skills ?? []).map((sk) => (
                  <tr key={sk.id}>
                    <td>
                      <code className="font-code text-xs">{sk.id}</code>
                    </td>
                    <td>
                      <Link
                        to="/agents/$skillId"
                        params={{ skillId: sk.id }}
                        className="font-medium no-underline hover:underline"
                      >
                        {sk.name}
                      </Link>
                      {sk.description && (
                        <div className="text-xs text-secondary mt-0.5">{sk.description}</div>
                      )}
                    </td>
                    <td className="text-right">{sk.projects_count}</td>
                  </tr>
                ))}
                {!skills.isLoading && (skills.data?.skills ?? []).length === 0 && (
                  <tr>
                    <td colSpan={3} className="text-secondary text-center py-6">
                      {t("agents.emptySkills")}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </SectionCard>
      </div>
    </>
  );
}
