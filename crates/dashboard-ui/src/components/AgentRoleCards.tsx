import type { AgentUsageStat } from "@/api/types";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const ROLES = [
  { id: "builder", icon: "construction", agents: ["coder", "builder", "default", "agent"] },
  { id: "verifier", icon: "fact_check", agents: ["verifier", "qa", "test"] },
  { id: "reviewer", icon: "rate_review", agents: ["reviewer", "review"] },
  { id: "planner", icon: "psychology", agents: ["planner", "plan", "architect"] },
] as const;

interface Props {
  agents: AgentUsageStat[];
}

export function AgentRoleCards({ agents }: Props) {
  const t = useT();

  return (
    <SectionCard title={t("agents.roleCards")}>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {ROLES.map((role) => {
          const match = agents.find((a) =>
            role.agents.some((key) => a.agent_type.toLowerCase().includes(key)),
          );
          const active = match && match.sessions_count > 0;
          return (
            <div
              key={role.id}
              className="border border-outline-variant rounded-lg p-4 bg-surface-container-low flex gap-3"
            >
              <div
                className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${
                  active ? "bg-primary/10 text-primary" : "bg-surface-container text-secondary"
                }`}
              >
                <Icon name={role.icon} size={22} />
              </div>
              <div className="min-w-0">
                <div className="font-medium text-on-surface">{t(`agents.roles.${role.id}`)}</div>
                <div className="text-xs text-secondary mt-0.5">
                  {match
                    ? `${match.agent_type} · ${match.model || "—"} · ${match.sessions_count} ${t("agents.sessionsShort")}`
                    : t("agents.roleIdle")}
                </div>
                <span
                  className={`inline-block mt-2 text-[10px] uppercase font-semibold px-2 py-0.5 rounded ${
                    active
                      ? "bg-success-container text-success"
                      : "bg-surface-container-high text-secondary"
                  }`}
                >
                  {active ? t("agents.roleActive") : t("agents.roleStandby")}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </SectionCard>
  );
}
