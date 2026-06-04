import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { AgentUsageStat, SkillRecord } from "@/api/types";
import { AgentRoleCards } from "@/components/AgentRoleCards";
import { EmptyState } from "@/components/EmptyState";
import { SkillSuggestionsPanel } from "@/components/SkillSuggestionsPanel";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/ui/PageHeader";
import { builtinAgentMeta } from "@/lib/agentCatalog";
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

function formatShortTime(iso: string | null): string {
  if (!iso) return "—";
  const normalized = iso.includes("T") ? iso : iso.replace(" ", "T");
  const d = new Date(normalized);
  if (Number.isNaN(d.getTime())) return iso.slice(0, 16);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
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
  const suggestions = useQuery({
    queryKey: ["skill-suggestions"],
    queryFn: api.skillSuggestions,
  });
  const rescan = useMutation({
    mutationFn: api.rescanSkills,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
      queryClient.invalidateQueries({ queryKey: ["skill-suggestions"] });
    },
  });
  const installStarter = useMutation({
    mutationFn: api.installStarterSkills,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills"] });
      queryClient.invalidateQueries({ queryKey: ["skill-suggestions"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
    },
  });

  const agentRows = aggregateAgentStats(stats.data?.agents ?? []);
  const skillList = skills.data?.skills ?? [];
  const missingStarter = suggestions.data?.missing_starter ?? [];
  const totalSessions = agentRows.reduce((n, r) => n + r.sessions_count, 0);
  const activeAgentTypes = agentRows.filter((r) => r.sessions_count > 0).length;
  const maxSessions = Math.max(1, ...agentRows.map((r) => r.sessions_count));

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
          <nav className="dw-agents-quick-nav" aria-label={t("agents.configureTitle")}>
            <Link to="/settings" search={{ section: "agents" }} className="dw-agents-quick-nav__item">
              <Icon name="tune" size={16} />
              {t("agents.configLink")}
            </Link>
            <Link to="/settings" search={{ section: "model" }} className="dw-agents-quick-nav__item">
              <Icon name="route" size={16} />
              {t("agents.routingLink")}
            </Link>
            <Link to="/settings" search={{ section: "skills" }} className="dw-agents-quick-nav__item">
              <Icon name="extension" size={16} />
              {t("agents.skillsLink")}
            </Link>
          </nav>
        }
      />

      <div className="dw-agents-page">
        <div className="dw-agents-kpi-strip">
          <KpiChip
            icon="extension"
            label={t("agents.skills")}
            value={skills.isLoading ? "…" : String(skillList.length)}
          />
          <KpiChip
            icon="smart_toy"
            label={t("agents.summaryActiveAgents")}
            value={String(activeAgentTypes)}
          />
          <KpiChip
            icon="forum"
            label={t("agents.summarySessions")}
            value={String(totalSessions)}
            highlight
          />
          <KpiChip
            icon="folder"
            label={t("agents.summaryPaths")}
            value="2"
            hint={t("agents.summaryPathsHint")}
          />
        </div>

        <SkillSuggestionsPanel />

        <div className="dw-agents-split">
          <section className="dw-agents-panel" aria-labelledby="agents-builtin-heading">
            <header className="dw-agents-panel__head">
              <h2 id="agents-builtin-heading" className="dw-agents-panel__title">
                {t("agents.builtinCards")}
              </h2>
            </header>
            <div className="dw-agents-panel__body dw-agents-panel__body--flush-x">
              <AgentRoleCards agents={stats.data?.agents ?? []} />
            </div>

            <header className="dw-agents-panel__head dw-agents-panel__head--sub">
              <h3 className="dw-agents-panel__title">{t("agents.agentStats")}</h3>
              {agentRows[0] && (
                <span className="dw-agents-panel__meta font-code">{agentRows[0].agent_type}</span>
              )}
            </header>
            <div className="dw-agents-panel__body">
              {stats.isLoading ? (
                <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
              ) : agentRows.length === 0 ? (
                <EmptyState
                  title={t("agents.emptyUsage")}
                  icon="smart_toy"
                  compact
                  actions={
                    <Link to="/conversations" className="dw-btn-primary text-sm no-underline">
                      <Icon name="chat" size={16} />
                      {t("agents.startConversation")}
                    </Link>
                  }
                />
              ) : (
                <ul className="dw-agents-stat-list m-0 p-0 list-none">
                  {agentRows.map((row, index) => (
                    <AgentStatRow
                      key={row.agent_type}
                      row={row}
                      maxSessions={maxSessions}
                      top={index === 0 && row.sessions_count > 0}
                      topLabel={t("agents.topAgent")}
                    />
                  ))}
                </ul>
              )}
            </div>
          </section>

          <section className="dw-agents-panel" aria-labelledby="agents-skills-heading">
            <header className="dw-agents-panel__head">
              <div>
                <h2 id="agents-skills-heading" className="dw-agents-panel__title">
                  {t("agents.skills")}
                </h2>
                {!skills.isLoading && (
                  <p className="dw-agents-panel__sub m-0">
                    {skillList.length > 0
                      ? t("agents.skillsSyncedCount").replace("{n}", String(skillList.length))
                      : t("agents.skillsSyncedNone")}
                  </p>
                )}
              </div>
              <button
                type="button"
                className="dw-btn-secondary text-sm shrink-0"
                disabled={rescan.isPending}
                onClick={() => rescan.mutate()}
              >
                <Icon name="refresh" size={16} />
                {rescan.isPending ? t("agents.rescanning") : t("agents.rescan")}
              </button>
            </header>

            <div className="dw-agents-panel__body dw-agents-panel__body--list">
              {rescan.isSuccess && (
                <p className="dw-agents-toast m-0" role="status">
                  <Icon name="check_circle" size={16} className="text-success" />
                  {t("agents.rescanSuccess").replace("{n}", String(rescan.data.skills_synced))}
                </p>
              )}
              {skills.isLoading ? (
                <p className="text-sm text-secondary m-0 px-4 py-6">{t("common.loading")}</p>
              ) : skillList.length === 0 ? (
                <div className="px-4 py-2">
                  <EmptyState
                    title={t("agents.emptySkillsTitle")}
                    description={t("agents.emptySkills")}
                    icon="extension"
                    compact
                    actions={
                      <>
                        {missingStarter.length > 0 && (
                          <button
                            type="button"
                            className="dw-btn-primary text-sm"
                            disabled={installStarter.isPending}
                            onClick={() => installStarter.mutate()}
                          >
                            <Icon name="download" size={16} />
                            {installStarter.isPending
                              ? t("agents.rescanning")
                              : t("agents.installStarterBtn")}
                          </button>
                        )}
                        <button
                          type="button"
                          className="dw-btn-secondary text-sm"
                          disabled={rescan.isPending}
                          onClick={() => rescan.mutate()}
                        >
                          <Icon name="refresh" size={16} />
                          {t("agents.rescan")}
                        </button>
                      </>
                    }
                  />
                </div>
              ) : (
                <ul className="dw-agents-skill-list m-0 p-0 list-none">
                  {skillList.map((skill) => (
                    <SkillRow
                      key={skill.id}
                      skill={skill}
                      projectsLabel={t("agents.projectsCount")}
                    />
                  ))}
                </ul>
              )}
            </div>
          </section>
        </div>
      </div>
    </>
  );
}

function KpiChip({
  icon,
  label,
  value,
  highlight,
  hint,
}: {
  icon: string;
  label: string;
  value: string;
  highlight?: boolean;
  hint?: string;
}) {
  return (
    <div className={`dw-agents-kpi-chip ${highlight ? "dw-agents-kpi-chip--hi" : ""}`}>
      <Icon name={icon} size={18} className="shrink-0 opacity-70" />
      <div className="min-w-0">
        <div className="dw-agents-kpi-chip__label">{label}</div>
        <div className="dw-agents-kpi-chip__value tabular-nums">{value}</div>
        {hint && <div className="dw-agents-kpi-chip__hint font-code">{hint}</div>}
      </div>
    </div>
  );
}

function AgentStatRow({
  row,
  maxSessions,
  top,
  topLabel,
}: {
  row: AgentUsageStat;
  maxSessions: number;
  top: boolean;
  topLabel: string;
}) {
  const t = useT();
  const meta = builtinAgentMeta(row.agent_type);
  const pct = Math.round((row.sessions_count / maxSessions) * 100);

  return (
    <li className="dw-agents-stat-row">
      <div
        className={`dw-agents-stat-row__icon ${
          row.sessions_count > 0 ? "text-primary bg-primary/10" : "text-secondary bg-surface-container-high"
        }`}
      >
        <Icon name={meta?.icon ?? "smart_toy"} size={18} />
      </div>
      <div className="dw-agents-stat-row__main min-w-0">
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-medium text-sm truncate">{row.agent_type}</span>
          {top && (
            <span className="dw-agents-stat-row__top">{topLabel}</span>
          )}
        </div>
        <span className="text-xs text-secondary font-code truncate block">{row.model}</span>
        <div className="dw-agents-stat-row__bar" aria-hidden>
          <span style={{ width: `${pct}%` }} />
        </div>
      </div>
      <div className="dw-agents-stat-row__nums shrink-0 text-right">
        <div className="text-lg font-semibold tabular-nums leading-none">{row.sessions_count}</div>
        <div className="text-[10px] text-secondary mt-0.5">{t("agents.sessionsShort")}</div>
      </div>
      <time className="dw-agents-stat-row__time hidden lg:block shrink-0 font-code text-[11px] text-secondary">
        {formatShortTime(row.last_started_at)}
      </time>
      <Link
        to="/conversations"
        search={{ agent: row.agent_type }}
        className="dw-agents-stat-row__go shrink-0"
        title="Open conversations"
      >
        <Icon name="arrow_forward" size={18} />
      </Link>
    </li>
  );
}

function SkillRow({ skill, projectsLabel }: { skill: SkillRecord; projectsLabel: string }) {
  return (
    <li>
      <Link to="/agents/$skillId" params={{ skillId: skill.id }} className="dw-agents-skill-row">
        <span className="dw-agents-skill-row__icon">
          <Icon name="extension" size={18} />
        </span>
        <span className="dw-agents-skill-row__body min-w-0">
          <span className="dw-agents-skill-row__name">{skill.name}</span>
          <span className="dw-agents-skill-row__id font-code">{skill.id}</span>
          {skill.description && (
            <span className="dw-agents-skill-row__desc">{skill.description}</span>
          )}
        </span>
        <span className="dw-agents-skill-row__meta shrink-0">
          <span className="tabular-nums">
            {skill.projects_count} {projectsLabel}
          </span>
          <Icon name="chevron_right" size={18} className="text-outline" />
        </span>
      </Link>
    </li>
  );
}
