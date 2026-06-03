import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { AgentUsageStat, SkillRecord } from "@/api/types";
import { AgentRoleCards } from "@/components/AgentRoleCards";
import { EmptyState } from "@/components/EmptyState";
import { SkillSuggestionsPanel } from "@/components/SkillSuggestionsPanel";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

function aggregateAgentStats(agents: AgentUsageStat[]): AgentUsageStat[] {
  const grouped = new Map<
    string,
    { sessions_count: number; models: Set<string>; last_started_at: string | null }
  >();

  for (const row of agents) {
    const current = grouped.get(row.agent_type) ?? {
      sessions_count: 0,
      models: new Set<string>(),
      last_started_at: null,
    };
    current.sessions_count += row.sessions_count;
    if (row.model) current.models.add(row.model);
    if (
      row.last_started_at &&
      (!current.last_started_at || row.last_started_at > current.last_started_at)
    ) {
      current.last_started_at = row.last_started_at;
    }
    grouped.set(row.agent_type, current);
  }

  return [...grouped.entries()]
    .map(([agent_type, value]) => {
      const models = [...value.models];
      return {
        agent_type,
        model:
          models.length === 0
            ? "—"
            : models.length <= 2
              ? models.join(", ")
              : `${models[0]}, ${models[1]} +${models.length - 2}`,
        sessions_count: value.sessions_count,
        last_started_at: value.last_started_at,
      };
    })
    .sort((a, b) => b.sessions_count - a.sessions_count);
}

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

  const agentRows = aggregateAgentStats(stats.data?.agents ?? []);
  const skillList = skills.data?.skills ?? [];

  return (
    <>
      <PageHeader
        title={t("agents.title")}
        subtitle={t("agents.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("agents.title") },
        ]}
        actions={
          <div className="dw-inline-links">
            <Link to="/settings" search={{ section: "agents" }} className="dw-inline-link">
              <Icon name="tune" size={16} />
              {t("agents.configLink")}
            </Link>
            <Link to="/settings" search={{ section: "model" }} className="dw-inline-link">
              <Icon name="route" size={16} />
              {t("agents.routingLink")}
            </Link>
            <Link to="/settings" search={{ section: "skills" }} className="dw-inline-link">
              <Icon name="extension" size={16} />
              {t("agents.skillsLink")}
            </Link>
          </div>
        }
      />

      <AgentRoleCards agents={stats.data?.agents ?? []} />

      <SkillSuggestionsPanel />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SectionCard title={t("agents.agentStats")}>
          {stats.isLoading ? (
            <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
          ) : agentRows.length === 0 ? (
            <EmptyState title={t("agents.emptyUsage")} icon="smart_toy" />
          ) : (
            <div className="space-y-2">
              {agentRows.map((row) => (
                <div
                  key={row.agent_type}
                  className="flex items-center gap-3 rounded-xl bg-surface-container-low px-3 py-2.5"
                >
                  <div className="w-8 h-8 rounded-lg bg-surface-container-high text-secondary flex items-center justify-center shrink-0">
                    <Icon name="smart_toy" size={18} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="font-medium text-sm truncate">{row.agent_type}</div>
                    <div className="text-xs text-secondary font-code truncate">{row.model}</div>
                  </div>
                  <div className="text-right shrink-0">
                    <div className="text-sm font-semibold tabular-nums">{row.sessions_count}</div>
                    <div className="text-[11px] text-secondary">{t("agents.sessionsShort")}</div>
                  </div>
                  <div className="hidden sm:block text-xs text-secondary shrink-0 w-36 text-right truncate">
                    {row.last_started_at ?? "—"}
                  </div>
                </div>
              ))}
            </div>
          )}
        </SectionCard>

        <SectionCard
          title={t("agents.skills")}
          action={
            <button
              type="button"
              className="dw-btn-ghost"
              disabled={rescan.isPending}
              onClick={() => rescan.mutate()}
            >
              <Icon name="refresh" size={16} />
              {rescan.isPending ? t("agents.rescanning") : t("agents.rescan")}
            </button>
          }
        >
          {rescan.isSuccess && (
            <p className="text-sm text-secondary m-0 mb-3">
              {t("agents.rescanSuccess").replace("{n}", String(rescan.data.skills_synced))}
            </p>
          )}
          {skills.isLoading ? (
            <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
          ) : skillList.length === 0 ? (
            <EmptyState
              title={t("agents.emptySkillsTitle")}
              description={t("agents.emptySkills")}
              icon="extension"
            />
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              {skillList.map((skill) => (
                <SkillTile key={skill.id} skill={skill} projectsLabel={t("agents.projectsCount")} />
              ))}
            </div>
          )}
        </SectionCard>
      </div>
    </>
  );
}

function SkillTile({ skill, projectsLabel }: { skill: SkillRecord; projectsLabel: string }) {
  return (
    <Link
      to="/agents/$skillId"
      params={{ skillId: skill.id }}
      className="dw-skill-tile group"
    >
      <div className="flex items-start gap-3">
        <div className="w-9 h-9 rounded-lg bg-primary/10 text-primary flex items-center justify-center shrink-0">
          <Icon name="extension" size={18} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="font-medium text-sm text-on-surface group-hover:text-primary transition-colors truncate">
            {skill.name}
          </div>
          <div className="text-[11px] font-code text-secondary truncate mt-0.5">{skill.id}</div>
          {skill.description && (
            <p className="text-xs text-secondary m-0 mt-2 line-clamp-2">{skill.description}</p>
          )}
        </div>
      </div>
      <div className="mt-3 text-xs text-secondary">
        {skill.projects_count} {projectsLabel}
      </div>
    </Link>
  );
}
