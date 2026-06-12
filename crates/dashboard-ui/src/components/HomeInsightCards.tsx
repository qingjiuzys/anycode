import { Link } from "@tanstack/react-router";
import type { DeliveryReadiness, OverviewStats } from "@/api/types";
import { buildConversationsHref } from "@/lib/conversationsSearch";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

interface Props {
  overview?: OverviewStats;
  readiness?: DeliveryReadiness;
  firstProjectId?: string;
}

export function HomeInsightCards({ overview, readiness, firstProjectId }: Props) {
  const t = useT();
  if (!overview) return null;

  const automationRate =
    overview.sessions_total > 0
      ? Math.round(
          ((overview.sessions_total - overview.sessions_blocked) / overview.sessions_total) * 100,
        )
      : 100;

  const risks: {
    label: string;
    to: string;
    trust?: "unverified";
  }[] = [];
  if (overview.sessions_blocked > 0) {
    risks.push({
      label: t("home.insightBlocked").replace("{n}", String(overview.sessions_blocked)),
      to: buildConversationsHref({ filter: "blocked" }),
    });
  }
  if (overview.gates_failed > 0) {
    risks.push({
      label: t("home.insightGates").replace("{n}", String(overview.gates_failed)),
      to: "/projects",
    });
  }
  if (readiness && readiness.unverified_artifacts > 0) {
    risks.push({
      label: t("home.insightAssets").replace("{n}", String(readiness.unverified_artifacts)),
      to: "/assets",
      trust: "unverified",
    });
  }

  const suggestions: {
    label: string;
    to: string;
    params?: { projectId: string };
  }[] = [];
  if (overview.projects_count === 0) {
    suggestions.push({ label: t("home.suggestScan"), to: "/projects" });
  }
  if (overview.skills_count === 0) {
    suggestions.push({ label: t("home.suggestSkills"), to: "/agents" });
  }
  if (firstProjectId) {
    suggestions.push({
      label: t("home.suggestKnowledge"),
      to: "/projects/$projectId",
      params: { projectId: firstProjectId },
    });
  }
  if (overview.sessions_running > 0) {
    suggestions.push({
      label: t("home.suggestRunning"),
      to: buildConversationsHref({ filter: "running" }),
    });
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
      <SectionCard title={t("home.insightAutomation")}>
        <div className="flex items-end gap-2">
          <span className="text-3xl font-bold text-primary tabular-nums">{automationRate}%</span>
          <span className="text-sm text-secondary pb-1">{t("home.insightSuccessRate")}</span>
        </div>
        <p className="text-xs text-secondary m-0 mt-2">
          {t("home.insightSessions")
            .replace("{total}", String(overview.sessions_total))
            .replace("{running}", String(overview.sessions_running))}
        </p>
      </SectionCard>

      <SectionCard title={t("home.insightRisks")}>
        {risks.length === 0 ? (
          <p className="text-sm text-success m-0 flex items-center gap-2">
            <Icon name="check_circle" size={18} />
            {t("home.insightNoRisks")}
          </p>
        ) : (
          <ul className="m-0 p-0 list-none space-y-2">
            {risks.map((r) => (
              <li key={r.label}>
                <Link
                  to={r.to}
                  search={r.trust ? { trust: r.trust } : undefined}
                  className={`text-sm no-underline hover:underline ${
                    r.to.includes("filter=blocked")
                      ? "text-error"
                      : r.trust
                        ? "text-secondary"
                        : "text-warn"
                  }`}
                >
                  {r.label}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </SectionCard>

      <SectionCard title={t("home.insightSuggestions")}>
        {suggestions.length === 0 ? (
          <p className="text-sm text-secondary m-0">{t("home.insightAllGood")}</p>
        ) : (
          <ul className="m-0 p-0 list-none space-y-2">
            {suggestions.map((s) => (
              <li key={s.label}>
                <Link
                  to={s.to}
                  params={s.params}
                  className="text-sm text-primary no-underline hover:underline"
                >
                  {s.label}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </SectionCard>
    </div>
  );
}
