import { Link } from "@tanstack/react-router";
import type { AgentUsageStat } from "@/api/types";
import type { AgentProfileRecord } from "@/api/types/agents";
import { AgentRoleCards } from "@/components/AgentRoleCards";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { builtinAgentMeta, agentDisplayLabel } from "@/lib/agentCatalog";
import { useT } from "@/i18n/context";

function formatShortTime(iso: string | null): string {
  if (!iso) return "—";
  const normalized = iso.includes("T") ? iso : iso.replace(" ", "T");
  const d = new Date(normalized);
  if (Number.isNaN(d.getTime())) return iso.slice(0, 16);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

type Props = {
  open: boolean;
  onClose: () => void;
  statsSummary: string;
  skillsLoading: boolean;
  skillCount: number;
  scanRoots: number;
  activeAgentTypes: number;
  totalSessions: number;
  rawAgents: AgentUsageStat[];
  agentRows: AgentUsageStat[];
  maxSessions: number;
  statsLoading: boolean;
  profilesLoading: boolean;
  customProfiles: AgentProfileRecord[];
  onSelectAgent: (agentId: string) => void;
  onNewAgent: () => void;
  onEditProfile: (id: string) => void;
};

export function AgentUsageStatsModal({
  open,
  onClose,
  statsSummary,
  skillsLoading,
  skillCount,
  scanRoots,
  activeAgentTypes,
  totalSessions,
  rawAgents,
  agentRows,
  maxSessions,
  statsLoading,
  profilesLoading,
  customProfiles,
  onSelectAgent,
  onNewAgent,
  onEditProfile,
}: Props) {
  const t = useT();

  return (
    <ModalOverlay
      open={open}
      onClose={onClose}
      labelledBy="agents-stats-modal-title"
      className="w-full max-w-3xl"
    >
      <div className="glass-modal rounded-xl flex flex-col max-h-[min(90dvh,820px)] overflow-hidden">
        <header className="flex items-start justify-between gap-3 px-5 py-4 border-b border-outline-variant/40 shrink-0">
          <div className="min-w-0">
            <h2 id="agents-stats-modal-title" className="text-lg font-semibold m-0 text-on-surface">
              {t("agents.statsSection")}
            </h2>
            <p className="text-[13px] text-secondary m-0 mt-1 tabular-nums">{statsSummary}</p>
          </div>
          <button
            type="button"
            className="dw-btn-ghost p-1 shrink-0"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <Icon name="close" size={20} />
          </button>
        </header>

        <div className="dw-agents-stats-modal__body overflow-y-auto overscroll-y-contain px-5 py-4 flex flex-col gap-4">
          <div className="dw-agents-kpi-strip dw-agents-kpi-strip--compact">
            <KpiChip
              icon="extension"
              label={t("agents.skills")}
              value={skillsLoading ? "…" : String(skillCount)}
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
              value={skillsLoading ? "…" : String(scanRoots)}
              hint={t("agents.summaryPathsHint")}
            />
          </div>

          <section className="dw-agents-stats-modal__section">
            <h3 className="dw-agents-stats-modal__heading">{t("agents.builtinCards")}</h3>
            <AgentRoleCards agents={rawAgents} onSelectAgent={onSelectAgent} />
          </section>

          <section className="dw-agents-stats-modal__section">
            <header className="dw-agents-stats-modal__section-head">
              <h3 className="dw-agents-stats-modal__heading m-0">{t("agents.agentStats")}</h3>
              {agentRows[0] && (
                <span className="dw-agents-panel__meta font-code">{agentRows[0].agent_type}</span>
              )}
            </header>
            {statsLoading ? (
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
                    onSelect={() => onSelectAgent(row.agent_type)}
                  />
                ))}
              </ul>
            )}
          </section>

          {!profilesLoading && customProfiles.length > 0 && (
            <section className="dw-agents-stats-modal__section">
              <header className="dw-agents-stats-modal__section-head">
                <h3 className="dw-agents-stats-modal__heading m-0">{t("agents.customAgents")}</h3>
                <button type="button" className="dw-btn-ghost text-[13px] shrink-0" onClick={onNewAgent}>
                  <Icon name="add" size={14} />
                  {t("agents.newAgent")}
                </button>
              </header>
              <ul className="m-0 p-0 list-none flex flex-col">
                {customProfiles.map((p) => (
                  <CustomProfileRow key={p.id} profile={p} onEdit={() => onEditProfile(p.id)} />
                ))}
              </ul>
            </section>
          )}
        </div>
      </div>
    </ModalOverlay>
  );
}

function CustomProfileRow({
  profile,
  onEdit,
}: {
  profile: AgentProfileRecord;
  onEdit: () => void;
}) {
  const t = useT();
  return (
    <li className="flex items-center gap-3 py-2 border-b border-outline-variant/30 last:border-b-0 min-w-0">
      <span className="dw-agents-stat-row__icon text-primary bg-primary/10 shrink-0">
        <Icon name="person" size={18} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="font-medium text-sm font-code truncate block">{profile.id}</span>
        <span className="text-[13px] text-secondary truncate block">
          {profile.description || profile.extends}
        </span>
      </span>
      <button type="button" className="dw-btn-ghost text-[13px] shrink-0" onClick={onEdit}>
        {t("common.details")}
      </button>
    </li>
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
  onSelect,
}: {
  row: AgentUsageStat;
  maxSessions: number;
  top: boolean;
  topLabel: string;
  onSelect: () => void;
}) {
  const t = useT();
  const meta = builtinAgentMeta(row.agent_type);
  const pct = Math.round((row.sessions_count / maxSessions) * 100);

  return (
    <li>
      <button type="button" className="dw-agents-stat-row dw-agents-stat-row--clickable" onClick={onSelect}>
        <div
          className={`dw-agents-stat-row__icon ${
            row.sessions_count > 0 ? "text-primary bg-primary/10" : "text-secondary bg-surface-container-high"
          }`}
        >
          <Icon name={meta?.icon ?? "smart_toy"} size={18} />
        </div>
        <div className="dw-agents-stat-row__main min-w-0">
          <div className="flex items-center gap-2 min-w-0">
            <span className="font-medium text-sm truncate">{agentDisplayLabel(row.agent_type, t)}</span>
            {top && <span className="dw-agents-stat-row__top">{topLabel}</span>}
          </div>
          <span className="text-[13px] text-secondary font-code truncate block">{row.model}</span>
          <div className="dw-agents-stat-row__bar" aria-hidden>
            <span style={{ width: `${pct}%` }} />
          </div>
        </div>
        <div className="dw-agents-stat-row__nums shrink-0 text-right">
          <div className="text-lg font-semibold tabular-nums leading-none">{row.sessions_count}</div>
          <div className="text-[13px] text-secondary mt-0.5">{t("agents.sessionsShort")}</div>
        </div>
        <time className="dw-agents-stat-row__time hidden sm:block shrink-0 font-code text-[13px] text-secondary">
          {formatShortTime(row.last_started_at)}
        </time>
        <span className="dw-agents-stat-row__go shrink-0 text-outline" aria-hidden>
          <Icon name="chevron_right" size={18} />
        </span>
      </button>
    </li>
  );
}
