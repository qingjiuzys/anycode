import { Link } from "@tanstack/react-router";
import { useMemo } from "react";
import type { AgentUsageStat } from "@/api/types";
import type { AgentProfileRecord } from "@/api/types/agents";
import { Icon } from "@/components/Icon";
import { BUILTIN_AGENT_CATALOG, builtinAgentMeta } from "@/lib/agentCatalog";
import { useT } from "@/i18n/context";

function formatShortTime(iso: string | null): string {
  if (!iso) return "—";
  const normalized = iso.includes("T") ? iso : iso.replace(" ", "T");
  const d = new Date(normalized);
  if (Number.isNaN(d.getTime())) return iso.slice(0, 16);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function aggregateForAgent(agentId: string, stats: AgentUsageStat[]) {
  const rows = stats.filter((r) => r.agent_type === agentId);
  const models = new Set<string>();
  let sessions = 0;
  let lastStarted: string | null = null;
  for (const row of rows) {
    sessions += row.sessions_count;
    if (row.model) models.add(row.model);
    if (row.last_started_at && (!lastStarted || row.last_started_at > lastStarted)) {
      lastStarted = row.last_started_at;
    }
  }
  return {
    sessions,
    models: [...models],
    lastStarted,
    modelLabel:
      models.size === 0
        ? "—"
        : models.size <= 2
          ? [...models].join(", ")
          : `${[...models][0]}, ${[...models][1]} +${models.size - 2}`,
  };
}

type Props = {
  agentId: string | null;
  stats: AgentUsageStat[];
  profiles?: AgentProfileRecord[];
  onClose: () => void;
  onEditProfile?: (profileId: string) => void;
};

export function AgentUsageDrawer({
  agentId,
  stats,
  profiles = [],
  onClose,
  onEditProfile,
}: Props) {
  const t = useT();

  const usage = useMemo(
    () => (agentId ? aggregateForAgent(agentId, stats) : null),
    [agentId, stats],
  );

  if (!agentId || !usage) return null;

  const meta = builtinAgentMeta(agentId);
  const catalogRole = BUILTIN_AGENT_CATALOG.find((r) => r.id === agentId);
  const displayName = catalogRole
    ? t(`agents.builtin.${catalogRole.labelKey}`)
    : agentId;
  const customProfile = profiles.find((p) => p.id === agentId && !p.builtin);

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-scrim/40" onClick={onClose}>
      <div
        className="w-full max-w-lg h-full bg-surface shadow-xl flex flex-col"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal
        aria-labelledby="agent-usage-drawer-title"
      >
        <div className="flex items-center justify-between p-4 border-b border-outline-variant">
          <h2 id="agent-usage-drawer-title" className="text-lg font-semibold m-0">
            {t("agents.drawerTitle")}
          </h2>
          <button type="button" className="dw-btn-ghost" onClick={onClose} aria-label={t("common.back")}>
            <Icon name="close" size={20} />
          </button>
        </div>

        <div className="p-4 space-y-5 overflow-y-auto flex-1">
          <div className="flex items-start gap-3">
            <span className="w-11 h-11 rounded-xl bg-primary/10 text-primary flex items-center justify-center shrink-0">
              <Icon name={meta?.icon ?? "smart_toy"} size={22} />
            </span>
            <div className="min-w-0">
              <div className="text-base font-semibold text-on-surface">{displayName}</div>
              <div className="text-xs font-code text-secondary mt-0.5">{agentId}</div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="dw-stat-card">
              <div className="dw-stat-value tabular-nums">{usage.sessions}</div>
              <div className="dw-stat-label">{t("agents.sessionsShort")}</div>
            </div>
            <div className="dw-stat-card">
              <div className="dw-stat-value text-sm font-code truncate" title={usage.modelLabel}>
                {usage.modelLabel}
              </div>
              <div className="dw-stat-label">{t("agents.drawerModels")}</div>
            </div>
          </div>

          <div>
            <div className="text-xs font-medium text-secondary mb-1">{t("agents.drawerLastUsed")}</div>
            <div className="text-sm font-code text-on-surface">{formatShortTime(usage.lastStarted)}</div>
          </div>

          {catalogRole && (
            <p className="text-sm text-secondary m-0">{t("agents.drawerBuiltinHint")}</p>
          )}
        </div>

        <div className="p-4 border-t border-outline-variant flex flex-col sm:flex-row gap-2">
          {customProfile && onEditProfile ? (
            <button
              type="button"
              className="dw-btn-primary flex-1"
              onClick={() => {
                onEditProfile(customProfile.id);
                onClose();
              }}
            >
              {t("agents.configureAgent")}
            </button>
          ) : (
            <Link
              to="/settings"
              search={{ section: "agents" }}
              className="dw-btn-primary flex-1 text-center no-underline"
              onClick={onClose}
            >
              {t("agents.configureAgent")}
            </Link>
          )}
          <Link
            to="/conversations"
            search={{ agent: agentId }}
            className="dw-btn-secondary flex-1 text-center no-underline"
            onClick={onClose}
          >
            <Icon name="forum" size={16} className="inline mr-1" />
            {t("agents.viewSessions")}
          </Link>
        </div>
      </div>
    </div>
  );
}
