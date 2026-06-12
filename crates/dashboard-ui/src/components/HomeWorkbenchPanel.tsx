import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { OverviewStats, ProjectSummary } from "@/api/types";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  overview?: OverviewStats;
  projects: ProjectSummary[];
  loadingProjects?: boolean;
  pendingApprovals: number;
  onNewProject: () => void;
};

export function HomeWorkbenchPanel({
  overview,
  projects,
  loadingProjects,
  pendingApprovals,
  onNewProject,
}: Props) {
  const t = useT();
  const qc = useQueryClient();
  const recentEvents = useQuery({
    queryKey: ["recent-events", "home-workbench"],
    queryFn: api.recentEvents,
    staleTime: 30_000,
  });
  const scan = useMutation({
    mutationFn: api.scanProjects,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["projects"] });
      void qc.invalidateQueries({ queryKey: ["overview"] });
      void qc.invalidateQueries({ queryKey: ["bootstrap"] });
    },
  });

  const recentProjects = projects.slice(0, 3);
  const latestError = (recentEvents.data?.events ?? []).find(
    (event) => event.severity === "error" || event.severity === "warn",
  );
  const pendingItems = [
    {
      key: "blocked",
      icon: "block",
      label: t("home.workbench.blockedSessions"),
      value: overview?.sessions_blocked ?? 0,
      tone: "error",
      to: "/conversations" as const,
      search: { trusted: "blocked" },
    },
    {
      key: "approvals",
      icon: "verified_user",
      label: t("home.workbench.pendingApprovals"),
      value: pendingApprovals,
      tone: "warn",
      to: "/conversations" as const,
      search: { needs_approval: true },
    },
    {
      key: "budget",
      icon: "speed",
      label: t("home.workbench.budgetExceeded"),
      value: overview?.sessions_budget_exceeded ?? 0,
      tone: "warn",
      to: "/conversations" as const,
      search: { budget_exceeded: true },
    },
    {
      key: "errors",
      icon: "error",
      label: t("home.workbench.recentError"),
      value: latestError ? 1 : 0,
      tone: latestError?.severity === "error" ? "error" : "neutral",
      to: latestError?.project_id ? ("/projects/$projectId" as const) : ("/reports" as const),
      params: latestError?.project_id ? { projectId: latestError.project_id } : undefined,
      detail: latestError?.title,
    },
  ];

  return (
    <div className="dw-home-workbench">
      <div className="dw-home-workbench__grid">
        <section className="dw-home-panel" aria-labelledby="home-recent-heading">
          <header className="dw-home-panel__head">
            <div>
              <h2 id="home-recent-heading" className="dw-home-panel__title">
                {t("home.workbench.recentTitle")}
              </h2>
              <p className="dw-home-panel__sub">{t("home.workbench.recentSubtitle")}</p>
            </div>
            <Link to="/projects" className="dw-home-panel__link">
              {t("common.viewAll")}
            </Link>
          </header>
          <div className="dw-home-recent-list">
            {loadingProjects ? (
              <p className="dw-home-empty">{t("common.loading")}</p>
            ) : recentProjects.length === 0 ? (
              <p className="dw-home-empty">{t("home.noProjects")}</p>
            ) : (
              recentProjects.map((project) => (
                <Link
                  key={project.id}
                  to="/projects/$projectId"
                  params={{ projectId: project.id }}
                  className="dw-home-recent-item"
                >
                  <span className="dw-home-recent-item__icon">
                    <Icon name="folder" size={18} />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="dw-home-recent-item__name">{project.name}</span>
                    <span className="dw-home-recent-item__meta">
                      {project.sessions_count} {t("home.workbench.sessions")} ·{" "}
                      {project.artifacts_count} {t("home.workbench.artifacts")}
                    </span>
                  </span>
                  <Icon name="chevron_right" size={18} className="text-outline shrink-0" />
                </Link>
              ))
            )}
          </div>
        </section>

        <section className="dw-home-panel" aria-labelledby="home-pending-heading">
          <header className="dw-home-panel__head">
            <div>
              <h2 id="home-pending-heading" className="dw-home-panel__title">
                {t("home.workbench.pendingTitle")}
              </h2>
              <p className="dw-home-panel__sub">{t("home.workbench.pendingSubtitle")}</p>
            </div>
            <Link to="/conversations" className="dw-home-panel__link">
              {t("home.actionConversations")}
            </Link>
          </header>
          <div className="dw-home-pending-grid">
            {pendingItems.map((item) => {
              const content = (
                <>
                  <span className={`dw-home-pending-item__icon dw-home-pending-item__icon--${item.tone}`}>
                    <Icon name={item.icon} size={18} />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="dw-home-pending-item__label">{item.label}</span>
                    {item.detail && <span className="dw-home-pending-item__detail">{item.detail}</span>}
                  </span>
                  <span className={`dw-home-pending-item__value dw-home-pending-item__value--${item.tone}`}>
                    {item.value}
                  </span>
                </>
              );

              if (item.params) {
                return (
                  <Link
                    key={item.key}
                    to={item.to}
                    params={item.params}
                    className="dw-home-pending-item"
                  >
                    {content}
                  </Link>
                );
              }

              return (
                <Link
                  key={item.key}
                  to={item.to}
                  search={item.search}
                  className="dw-home-pending-item"
                >
                  {content}
                </Link>
              );
            })}
          </div>
        </section>
      </div>

      <div className="dw-home-action-bar" aria-label={t("home.quickActions")}>
        <button
          type="button"
          className="dw-home-action"
          disabled={scan.isPending}
          onClick={() => scan.mutate()}
        >
          <Icon name="radar" size={17} />
          {scan.isPending ? t("common.loading") : t("home.actionScan")}
        </button>
        <button type="button" className="dw-home-action" onClick={onNewProject}>
          <Icon name="add" size={17} />
          {t("home.actionNewProject")}
        </button>
        <Link to="/agents" className="dw-home-action">
          <Icon name="extension" size={17} />
          {t("home.workbench.importSkills")}
        </Link>
        <Link to="/settings" className="dw-home-action">
          <Icon name="settings" size={17} />
          {t("home.actionSettings")}
        </Link>
        <Link to="/reports" className="dw-home-action">
          <Icon name="description" size={17} />
          {t("home.workbench.viewReports")}
        </Link>
      </div>
    </div>
  );
}
