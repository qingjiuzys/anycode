import type { AgentUsageStat } from "@/api/types";
import { Link } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { BUILTIN_AGENT_CATALOG } from "@/lib/agentCatalog";
import { useT } from "@/i18n/context";

interface Props {
  agents: AgentUsageStat[];
}

export function AgentRoleCards({ agents }: Props) {
  const t = useT();

  const roles = [...BUILTIN_AGENT_CATALOG]
    .map((role) => {
      const matches = agents.filter((a) => a.agent_type === role.id);
      const sessions = matches.reduce((sum, row) => sum + row.sessions_count, 0);
      return { role, sessions, active: sessions > 0 };
    })
    .sort((a, b) => b.sessions - a.sessions || Number(b.active) - Number(a.active));

  return (
    <div className="dw-agents-builtin-scroll" role="list">
      {roles.map(({ role, sessions, active }) => (
        <Link
          key={role.id}
          to="/conversations"
          search={{ agent: role.id }}
          role="listitem"
          className={`dw-agents-builtin-chip ${active ? "dw-agents-builtin-chip--active" : ""}`}
        >
          <span
            className={`dw-agents-builtin-chip__icon ${
              active ? "text-primary bg-primary/10" : "text-secondary bg-surface-container-high"
            }`}
          >
            <Icon name={role.icon} size={18} />
          </span>
          <span className="dw-agents-builtin-chip__body">
            <span className="dw-agents-builtin-chip__name">
              {t(`agents.builtin.${role.labelKey}`)}
            </span>
            <span className="dw-agents-builtin-chip__id font-code">{role.id}</span>
          </span>
          <span
            className={`dw-agents-builtin-chip__badge ${
              active ? "dw-agents-builtin-chip__badge--on" : ""
            }`}
          >
            {active ? (
              <>
                <span className="tabular-nums">{sessions}</span>
                <span className="opacity-80">{t("agents.sessionsShort")}</span>
              </>
            ) : (
              t("agents.roleStandby")
            )}
          </span>
        </Link>
      ))}
    </div>
  );
}
