import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { api } from "@/api/client";
import type { AgentUsageStat } from "@/api/types";
import { AgentUsageDrawer } from "@/components/AgentUsageDrawer";
import { AgentUsageStatsModal } from "@/components/AgentUsageStatsModal";
import { InstalledSkillsPanel } from "@/components/InstalledSkillsPanel";
import { SkillMarketPanel } from "@/components/SkillMarketPanel";
import { SkillSuggestionsPanel } from "@/components/SkillSuggestionsPanel";
import { SkillsImportPanel } from "@/components/SkillsImportPanel";
import { AgentEditorDrawer } from "@/components/settings/AgentEditorDrawer";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { PageHeader } from "@/components/ui/PageHeader";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";

type SkillsTab = "installed" | "catalog" | "import";

function isMockModel(model: string | null | undefined): boolean {
  const m = (model ?? "").trim().toLowerCase();
  return m === "mock" || m.startsWith("mock/");
}

function aggregateAgentStats(agents: AgentUsageStat[]): AgentUsageStat[] {
  const grouped = new Map<
    string,
    { sessions_count: number; models: Set<string>; last_started_at: string | null }
  >();

  for (const row of agents) {
    if (isMockModel(row.model)) continue;
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

export function AgentsPage(_props: EmbeddedPageProps = {}) {
  const t = useT();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<SkillsTab>("installed");
  const [statsOpen, setStatsOpen] = useState(false);
  const stats = useQuery({
    queryKey: ["agent-stats"],
    queryFn: () => api.agentStats(30),
  });
  const skills = useQuery({
    queryKey: ["skills"],
    queryFn: () => api.skills(200),
  });
  const suggestions = useQuery({
    queryKey: ["skill-suggestions"],
    queryFn: api.skillSuggestions,
  });
  const profiles = useQuery({
    queryKey: ["agent-profiles"],
    queryFn: () => api.agentProfiles(),
  });
  const [editor, setEditor] = useState<{ id?: string } | null>(null);
  const [agentDrawer, setAgentDrawer] = useState<string | null>(null);
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

  const rawAgents = (stats.data?.agents ?? []).filter((a) => !isMockModel(a.model));
  const agentRows = aggregateAgentStats(stats.data?.agents ?? []);
  const skillList = skills.data?.skills ?? [];
  const customProfiles = (profiles.data?.profiles ?? []).filter((p) => !p.builtin);
  const allProfiles = profiles.data?.profiles ?? [];
  const missingStarter = suggestions.data?.missing_starter ?? [];
  const totalSessions = agentRows.reduce((n, r) => n + r.sessions_count, 0);
  const activeAgentTypes = agentRows.filter((r) => r.sessions_count > 0).length;
  const maxSessions = Math.max(1, ...agentRows.map((r) => r.sessions_count));

  const statsSummary = t("agents.statsSectionSummary")
    .replace("{skills}", skills.isLoading ? "…" : String(skillList.length))
    .replace("{agents}", String(activeAgentTypes))
    .replace("{sessions}", String(totalSessions));

  const tabs: { id: SkillsTab; label: string }[] = [
    { id: "installed", label: t("agents.tabs.installed") },
    { id: "catalog", label: t("agents.tabs.catalog") },
    { id: "import", label: t("agents.tabs.import") },
  ];

  function openAgentDetail(agentId: string) {
    setStatsOpen(false);
    setAgentDrawer(agentId);
  }

  function openNewAgent() {
    setStatsOpen(false);
    setEditor({});
  }

  function openEditProfile(id: string) {
    setStatsOpen(false);
    setEditor({ id });
  }

  return (
    <>
      <CcPageShell
        header={
          <>
            <PageHeader
              title={t("agents.title")}
              subtitle={t("agents.subtitle")}
              breadcrumbs={[
                { label: t("nav.home"), to: "/" },
                { label: t("agents.title") },
              ]}
              actions={
                <nav className="dw-agents-quick-nav" aria-label={t("agents.configureTitle")}>
                  <button
                    type="button"
                    className="dw-agents-quick-nav__item"
                    onClick={() => setStatsOpen(true)}
                    aria-haspopup="dialog"
                  >
                    <Icon name="bar_chart" size={16} />
                    <span className="hidden sm:inline">{t("agents.usage")}</span>
                    <span className="text-[13px] font-semibold tabular-nums text-secondary sm:ml-0.5">
                      {skills.isLoading ? "…" : skillList.length}/{totalSessions}
                    </span>
                  </button>
                  <button
                    type="button"
                    className="dw-btn-primary text-sm"
                    onClick={() => setEditor({})}
                  >
                    <Icon name="add" size={16} />
                    {t("agents.newAgent")}
                  </button>
                  <Link to="/settings" search={{ section: "agents" }} className="dw-agents-quick-nav__item">
                    <Icon name="tune" size={16} />
                    {t("agents.configLink")}
                  </Link>
                  <Link to="/settings" search={{ section: "model" }} className="dw-agents-quick-nav__item">
                    <Icon name="route" size={16} />
                    {t("agents.routingLink")}
                  </Link>
                </nav>
              }
            />
            <div className="dw-agents-tabs mt-3" role="tablist" aria-label={t("agents.skills")}>
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  type="button"
                  role="tab"
                  id={`agents-tab-${tab.id}`}
                  aria-selected={activeTab === tab.id}
                  aria-controls={`agents-panel-${tab.id}`}
                  className={`dw-agents-tabs__tab ${activeTab === tab.id ? "dw-agents-tabs__tab--active" : ""}`}
                  onClick={() => setActiveTab(tab.id)}
                >
                  {tab.label}
                </button>
              ))}
            </div>
          </>
        }
      >
        <div className="dw-agents-page">
          <SkillSuggestionsPanel />

          <section className="dw-agents-skills-shell" aria-label={t("agents.skills")}>
            <div className="dw-agents-tab-content">
              {activeTab === "installed" && (
                <div
                  role="tabpanel"
                  id="agents-panel-installed"
                  aria-labelledby="agents-tab-installed"
                  className="dw-agents-tab-panel-wrap"
                >
                  <InstalledSkillsPanel
                    embedded
                    skills={skillList}
                    loading={skills.isLoading}
                    rescanPending={rescan.isPending}
                    onRescan={() => rescan.mutate()}
                    rescanSuccess={rescan.isSuccess ? rescan.data.skills_synced : undefined}
                    missingStarterCount={missingStarter.length}
                    onInstallStarter={() => installStarter.mutate()}
                    installStarterPending={installStarter.isPending}
                  />
                </div>
              )}
              {activeTab === "catalog" && (
                <div
                  role="tabpanel"
                  id="agents-panel-catalog"
                  aria-labelledby="agents-tab-catalog"
                  className="dw-agents-tab-panel-wrap"
                >
                  <p className="text-[13px] text-secondary m-0 mb-3">{t("agents.skillMarketHint")}</p>
                  <SkillMarketPanel embedded />
                </div>
              )}
              {activeTab === "import" && (
                <div
                  role="tabpanel"
                  id="agents-panel-import"
                  aria-labelledby="agents-tab-import"
                  className="dw-agents-tab-panel-wrap"
                >
                  <SkillsImportPanel />
                </div>
              )}
            </div>
          </section>
        </div>
      </CcPageShell>

      <AgentUsageStatsModal
        open={statsOpen}
        onClose={() => setStatsOpen(false)}
        statsSummary={statsSummary}
        skillsLoading={skills.isLoading}
        skillCount={skillList.length}
        scanRoots={skills.data?.scan_roots ?? 0}
        activeAgentTypes={activeAgentTypes}
        totalSessions={totalSessions}
        rawAgents={rawAgents}
        agentRows={agentRows}
        maxSessions={maxSessions}
        statsLoading={stats.isLoading}
        profilesLoading={profiles.isLoading}
        customProfiles={customProfiles}
        onSelectAgent={openAgentDetail}
        onNewAgent={openNewAgent}
        onEditProfile={openEditProfile}
      />

      <AgentUsageDrawer
        agentId={agentDrawer}
        stats={stats.data?.agents ?? []}
        profiles={allProfiles}
        onClose={() => setAgentDrawer(null)}
        onEditProfile={(id) => setEditor({ id })}
      />

      {editor !== null && (
        <AgentEditorDrawer
          profileId={editor.id}
          onClose={() => setEditor(null)}
          onSaved={() => {
            setEditor(null);
            queryClient.invalidateQueries({ queryKey: ["agent-profiles"] });
          }}
        />
      )}
    </>
  );
}
