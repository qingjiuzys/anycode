import type { AgentUsageStat } from "@/api/types";
import { Link } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const BUILTIN_AGENTS = [
  { id: "general-purpose", icon: "smart_toy", labelKey: "generalPurpose" },
  { id: "explore", icon: "travel_explore", labelKey: "explore" },
  { id: "plan", icon: "psychology", labelKey: "plan" },
  { id: "workspace-assistant", icon: "hub", labelKey: "workspaceAssistant" },
  { id: "goal", icon: "flag", labelKey: "goal" },
  { id: "office-writer", icon: "edit_note", labelKey: "officeWriter" },
  { id: "data-analyst", icon: "table_chart", labelKey: "dataAnalyst" },
  { id: "researcher", icon: "science", labelKey: "researcher" },
  { id: "file-operator", icon: "folder_open", labelKey: "fileOperator" },
] as const;

interface Props {
  agents: AgentUsageStat[];
}

export function AgentRoleCards({ agents }: Props) {
  const t = useT();

  return (
    <SectionCard title={t("agents.builtinCards")}>
      <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-3">
        {BUILTIN_AGENTS.map((role) => {
          const matches = agents.filter((a) => a.agent_type === role.id);
          const sessions = matches.reduce((sum, row) => sum + row.sessions_count, 0);
          const active = sessions > 0;
          const models = [...new Set(matches.map((row) => row.model).filter(Boolean))];
          const modelLabel =
            models.length === 0
              ? "—"
              : models.length === 1
                ? models[0]
                : `${models[0]} +${models.length - 1}`;

          return (
            <div key={role.id} className="dw-agent-tile">
              <div className="flex items-start justify-between gap-2">
                <div
                  className={`dw-agent-tile-icon ${
                    active ? "bg-primary/10 text-primary" : "bg-surface-container-high text-secondary"
                  }`}
                >
                  <Icon name={role.icon} size={20} />
                </div>
                <span
                  className={`text-[10px] uppercase font-semibold px-2 py-0.5 rounded-full shrink-0 ${
                    active
                      ? "bg-success-container text-success"
                      : "bg-surface-container-high text-secondary"
                  }`}
                >
                  {active ? t("agents.roleActive") : t("agents.roleStandby")}
                </span>
              </div>
              <div className="min-w-0">
                <div className="font-medium text-sm text-on-surface truncate">
                  {t(`agents.builtin.${role.labelKey}`)}
                </div>
                <div className="text-[11px] text-secondary mt-0.5 font-code truncate">{role.id}</div>
              </div>
              <div className="text-xs text-secondary min-h-[2rem]">
                {active ? (
                  <>
                    <div className="font-code truncate" title={models.join(", ")}>
                      {modelLabel}
                    </div>
                    <div>
                      {sessions} {t("agents.sessionsShort")}
                    </div>
                  </>
                ) : (
                  t("agents.roleIdle")
                )}
              </div>
              <Link
                to="/conversations"
                search={{ agent: role.id, filter: "all" }}
                className="inline-flex items-center gap-1 text-xs text-primary no-underline hover:underline mt-auto"
              >
                {t("agents.startConversation")}
                <Icon name="arrow_forward" size={14} />
              </Link>
            </div>
          );
        })}
      </div>
    </SectionCard>
  );
}
